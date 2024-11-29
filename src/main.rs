use png::ColorType;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::str;

#[derive(Debug, Clone)]
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
        if args.len() < 4 {
            return Err("not enough arguments");
        }
        let action = match args[1].as_str() {
            "-r" => ActionType::Read,
            "-w" => ActionType::Write,
            _ => return Err("not enough atguments"),
        };
        let file_path = args[2].clone();
        let secret = Some(args[3].clone());

        Ok(Config {
            action,
            file_path,
            secret,
        })
    }
}

#[derive(Debug)]
struct FileHandler {}

impl FileHandler {
    fn build() -> Result<FileHandler, &'static str> {
        Ok(FileHandler {})
    }

    /// Записать текст в изображение
    fn write_secret(
        &self,
        file_name: &str,
        new_secret: Option<String>,
    ) -> Result<(), &'static str> {
        let old_path = Path::new(file_name);
        let decoder = png::Decoder::new(File::open(old_path).unwrap());
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();

        let new_path = Path::new("lol.png");
        let file = File::create(new_path).unwrap();
        let ref mut w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, info.width, info.height);

        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        let binding = new_secret.unwrap();
        let secret_data = binding.as_bytes();
        let secret_data_len = secret_data.len();

        let data_bytes: &[u8] = &buf[..info.buffer_size()];

        let mut outer_secret_index: usize = 0;
        let mut iner_secret_index: usize = 0;

        let mut data_bytes_mut = data_bytes.to_vec();
        for byte in &mut data_bytes_mut {
            if outer_secret_index != secret_data_len {
                let choose = (secret_data[outer_secret_index] >> iner_secret_index) & 1;
                match choose {
                    0b0 => *byte &= !(1 << 0),
                    0b1 => *byte |= 1 << 0,
                    _ => unreachable!(),
                };
                if iner_secret_index == 8 {
                    outer_secret_index += 1;
                    iner_secret_index = 0;
                }
                iner_secret_index += 1;
            }else {
                *byte |= 1 << 0;
            }
        }
        writer.write_image_data(&data_bytes_mut).unwrap(); // save
        Ok(())
    }

    fn get_secret(&self, file_name: &str) -> String {
        let path = Path::new(file_name);
        let decoder = png::Decoder::new(File::open(path).unwrap());
        let mut reader = decoder.read_info().unwrap();

        let mut buf = vec![0; reader.output_buffer_size()];

        let info = reader.next_frame(&mut buf).unwrap();

        let image_bytes = &buf[..info.buffer_size()];

        let mut buf: u8 = 0b0000_0000;
        let mut i: usize = 8;
        let mut buff_index: usize = 0;

        let mut secret_bin: Vec<u8> = vec![0; image_bytes.len() / 8];

        for byte in image_bytes {
            if i == 8 {
                secret_bin[buff_index] = buf;
                buff_index += 1;
                buf = 0b0000_0000;
                i = 0;
            }
            let choose = byte & 1;
            match choose {
                0b0 => buf &= !(1 << i),
                0b1 => buf |= 1 << i,
                _ => unreachable!(),
            };
            i += 1;
        }
        dbg!("info - {:?}", info);
        let secret_str = String::from_utf8_lossy(&secret_bin).to_string();
        secret_str
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let cfg = Config::build(&args);
    println!("{:?}", cfg);

    let config = cfg?;
    let file_handler = FileHandler::build()?;

    match config.action {
        ActionType::Write => {
            let _ = file_handler.write_secret(config.file_path.as_str(), config.secret);
        }
        ActionType::Read => {
            println!("{}", file_handler.get_secret(config.file_path.as_str()))
        }
    };
    Ok(())
}

// cargo run -- -w ferris.png "very secret token"
// cargo run -- -r code_string example-filename.png
