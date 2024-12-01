use png::OutputInfo;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::process;
use std::str;

#[derive(Debug, Clone, PartialEq)]
enum ActionType {
    Read,
    Write,
}

#[derive(Debug, Clone)]
struct Config {
    pub action: ActionType,
    file_path: String,
    secret: Option<String>,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        let action = match args[1].as_str() {
            "-r" => ActionType::Read,
            "-w" => ActionType::Write,
            _ => return Err("No read/write (-r/-w) flag on position '1'."),
        };
        if (action == ActionType::Read) && args.len() < 3
            || (action == ActionType::Write) && args.len() < 4
        {
            return Err("Not enough arguments");
        }
        let mut secret: Option<String> = None;
        let file_path = args[2].clone();
        if action == ActionType::Write {
            secret = Some(args[3].clone());
        }
        Ok(Config {
            action,
            file_path,
            secret,
        })
    }
}

trait HandlerAction {
    fn read_secret(&self) -> String;
    fn write_secret(&self, new_secret: Option<String>) -> Result<(), &'static str>;
}

#[derive(Debug)]
struct RGBFileHandler<'a> {
    path: &'a Path,
    info: OutputInfo,
    bytes: Vec<u8>,
}

impl RGBFileHandler<'_> {
    fn build(file_name: &str) -> Result<RGBFileHandler, &'static str> {
        let path = Path::new(file_name);
        let decoder = png::Decoder::new(File::open(path).unwrap_or_else(|err| {
            eprintln!("Application error: {err}");
            process::exit(1);
        }));
        let mut reader = decoder.read_info().unwrap();
        let mut bytes = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut bytes).unwrap();
        Ok(RGBFileHandler { path, info, bytes })
    }
}

/// Работает для png картинки с параметрами png::ColorType::Rgb, png::BitDepth::Eight.
impl HandlerAction for RGBFileHandler<'_> {
    /// Записать текст в изображение
    fn write_secret(&self, new_secret: Option<String>) -> Result<(), &'static str> {
        let file = File::create(self.path).unwrap();
        let w = &mut BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, self.info.width, self.info.height);

        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        let secret_data_raw = new_secret.unwrap();
        let secret_data = secret_data_raw.as_bytes();

        let mut outer_secret_index: usize = 0;
        let mut iner_secret_index: usize = 0;

        let secret_data_len = secret_data.len();
        let mut data_bytes_mut = self.bytes.to_vec();
        for byte in &mut data_bytes_mut {
            if outer_secret_index != secret_data_len {
                let choose = (secret_data[outer_secret_index] >> iner_secret_index) & 1;
                match choose {
                    0b0 => *byte &= !(1 << 0),
                    0b1 => *byte |= 1 << 0,
                    _ => unreachable!(),
                };
                iner_secret_index += 1;
                if iner_secret_index == 8 {
                    outer_secret_index += 1;
                    iner_secret_index = 0;
                }
            } else {
                *byte |= 1 << 0;
            }
        }
        writer.write_image_data(&data_bytes_mut).unwrap();
        println!("Done");
        Ok(())
    }

    /// Прочитать текст из изображения
    fn read_secret(&self) -> String {
        let mut i: usize = 8;
        let mut one_couner = 0;

        let mut secret_bin: Vec<u8> = Vec::new();
        let mut acc: u8 = 0b0000_0000;
        for byte in &self.bytes {
            if i == 8 {
                secret_bin.push(acc);
                acc = 0b0000_0000;
                i = 0;
            }
            let choose = byte & 1;
            match choose {
                0b0 => {
                    acc &= !(1 << i);
                    one_couner = 0;
                }
                0b1 => {
                    acc |= 1 << i;
                    one_couner += 1;
                }
                _ => unreachable!(),
            };
            i += 1;
            if one_couner == 8 {
                break;
            }
        }
        String::from_utf8(secret_bin).unwrap()
    }
}

/// # Run example:
/// cargo run --release -- -w ferris.png "very secret token"
/// cargo run --release -- -r ferris.png
///
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Application error: {err}");
        process::exit(1);
    });
    let file_handler = RGBFileHandler::build(config.file_path.as_str()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    match config.action {
        ActionType::Write => {
            let _ = file_handler.write_secret(config.secret);
        }
        ActionType::Read => {
            println!("{}", &file_handler.read_secret())
        }
    };
    Ok(())
}
