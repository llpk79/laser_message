use crate::huffman_code::HuffTree;
use gpio::GpioValue::{High, Low};
use gpio::{GpioIn, GpioOut};
use std::thread;
use std::time::Duration;

const LASER_PIN: u16 = 18;
const RECEIVER_PIN: u16 = 23;

pub struct Laser {
    out: gpio::sysfs::SysFsGpioOutput,
    encoded_message: Vec<u32>,
}

pub struct Receiver {
    in_: gpio::sysfs::SysFsGpioInput,
    huff_tree: HuffTree,
}

impl Laser {
    pub fn new(encoded_message: Vec<u32>) -> Laser {
        // Open port for laser pin.
        let out = match gpio::sysfs::SysFsGpioOutput::open(LASER_PIN) {
            Ok(out) => out,
            Err(_e) => panic!(),
        };
        Self {
            out,
            encoded_message,
        }
    }

    /// Initiate message with 500 microsecond pulse.
    ///
    /// Transmit message; long pulse = 1 short pulse = 0.
    ///
    /// Terminate message with 1000 microsecond pulse.
    pub fn send_message(&mut self) {
        // Initiation sequence.
        self.out.set_value(false).expect("Pin is on");
        thread::sleep(Duration::from_micros(50));
        self.out.set_value(true).expect("Pin is on");
        thread::sleep(Duration::from_micros(500));
        self.out.set_value(false).expect("Pin is on");
        thread::sleep(Duration::from_micros(50));

        // Begin message transmission.
        for bit in &self.encoded_message {
            match *bit == 1 {
                true => {
                    self.out.set_value(true).expect("Pin is on");
                    thread::sleep(Duration::from_micros(25));
                    self.out.set_value(false).expect("Pin is on");
                }
                false => {
                    self.out.set_value(true).expect("Pin is on");
                    thread::sleep(Duration::from_micros(10));
                    self.out.set_value(false).expect("Pin is on");
                }
            }
            // Bit resolution. It gets sloppy below 50 microseconds.
            thread::sleep(Duration::from_micros(50))
        }

        // Termination sequence.
        self.out.set_value(true).expect("Pin is on");
        thread::sleep(Duration::from_micros(1000));
        self.out.set_value(false).expect("Pin is on");
    }
}

impl Receiver {
    pub fn new(huff_tree: HuffTree) -> Receiver {
        // Open port for receiver pin.
        let in_ = match gpio::sysfs::SysFsGpioInput::open(RECEIVER_PIN) {
            Ok(in_) => in_,
            Err(_e) => panic!(),
        };
        Self { in_, huff_tree }
    }

    /// Loop until initiation sequence is detected.
    fn detect_message(&mut self) {
        loop {
            while self.in_.read_value().expect("Pin is on") == Low {
                continue;
            }
            // Get the amount of time the laser is on.
            let begin = chrono::Utc::now();
            while self.in_.read_value().expect("Pin is on") == High {
                continue;
            }
            let initiation_time = (chrono::Utc::now() - begin)
                .num_microseconds()
                .expect("Time has passed");
            match initiation_time {
                i64::MIN..=400 => continue,
                401..=900 => break,
                901.. => continue,
            }
        }
    }

    /// Push 1 for long pulse, 0 for short.
    ///
    /// Return data upon termination sequence.
    fn receive_message(&mut self) -> Vec<u32> {
        let mut data = Vec::new();
        loop {
            while self.in_.read_value().expect("Pin is on") == Low {
                continue;
            }
            // Get the amount of time the laser is on.
            let start = chrono::Utc::now();
            while self.in_.read_value().expect("Pin is on") == High {
                continue;
            }
            let bit_time = (chrono::Utc::now() - start)
                .num_microseconds()
                .expect("Time has passed");
            // println!("l bit time {}", bit_time);
            match bit_time {
                i64::MIN..=-0 => continue,
                1..=89 => data.push(0),
                90..=199 => data.push(1),
                200..=1000 => continue, // Bad data, we could guess, I guess?
                1001.. => break,        // Termination sequence.
            };
        }
        data
    }

    /// Call detect, receive and decode methods.
    ///
    /// Print to stdout.
    pub fn print_message(&mut self) {
        println!("\n\nAwaiting transmission...");
        self.detect_message();
        let start = chrono::Utc::now();

        println!("\nIncoming message detected...\n");
        let data = self.receive_message();
        let message = self.huff_tree.decode(&data);

        // Calculate stats
        let num_kbytes = message.len() as f64 / 1000.0;
        let seconds = (chrono::Utc::now() - start).num_milliseconds() as f64 / 1000.0_f64;

        println!("{message}");
        println!(
            "Message in {:.4} sec\nKB/s {:.3}\n",
            seconds,
            num_kbytes / seconds,
        );
    }
}

/// Send a message with a laser!
pub fn do_laser(message: String) {
    // Compress message with Huffman Coding.
    let mut huff_tree = HuffTree::new();
    let encoded_message = huff_tree.encode(message);

    // Pass huff_tree to receiver to decode message.
    let mut receiver = Receiver::new(huff_tree);
    let mut laser = Laser::new(encoded_message);

    // Start a thread each for the laser and receiver.
    let receiver_thread = thread::Builder::new()
        .name("receiver".to_string())
        .spawn(move || loop {
            receiver.print_message();
        });

    let laser_thread = thread::Builder::new()
        .name("laser".to_string())
        .spawn(move || loop {
            laser.send_message();
            thread::sleep(Duration::from_millis(2000))
        });

    receiver_thread
        .expect("Thread exists")
        .join()
        .expect("Thread closes");
    laser_thread
        .expect("Thread exists")
        .join()
        .expect("Thread closes");
}


