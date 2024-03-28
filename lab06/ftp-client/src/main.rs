use ftp::{FtpError, FtpStream};
use std::fs::File;
use std::io::{BufReader, BufWriter, Stdin, Write};
use std::path::Path;
use std::{env, io};

#[derive(Debug)]
enum Either<T1, T2> {
    Left(T1),
    Right(T2),
}

struct FtpClient {
    ftp_stream: FtpStream,
}

impl FtpClient {
    fn list_dir(&mut self, path: Option<&str>, ident: &str) -> Result<(), FtpError> {
        let list = self.ftp_stream.nlst(path)?;
        let new_ident: &str = &(ident.to_owned() + "\t");
        for file_name in list {
            let path = Path::new(&file_name);
            println!("{}{}", ident, path.file_name().unwrap().to_str().unwrap());

            if path.extension().is_none() {
                self.list_dir(Some(&file_name), new_ident)?;
            }
        }
        Ok(())
    }

    pub fn process_command(&mut self, stdin: &Stdin) -> Result<(), Either<FtpError, io::Error>> {
        let mut command = String::new();

        stdin
            .read_line(&mut command)
            .map_err(|err| Either::Right(err))?;
        match command.trim_end() {
            "ls" => self
                .list_dir(None, String::from("").as_str())
                .map_err(|err| Either::Left(err))?,
            "quit" => {
                return Err(Either::Right(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "End of stream",
                )));
            }
            command => {
                let args: Vec<&str> = command.split(" ").collect();
                match args[0] {
                    "put" => self.put(args)?,
                    "get" => self.get(args)?,
                    c => eprintln!("Unsupported command: {}", c),
                }
            }
        }
        command.clear();
        Ok(())
    }

    fn put(&mut self, args: Vec<&str>) -> Result<(), Either<FtpError, io::Error>> {
        let (local_file, remote_file) = (args[1], args[2]);
        let mut reader = BufReader::new(File::open(local_file).map_err(|err| Either::Right(err))?);
        Ok(self
            .ftp_stream
            .put(remote_file, &mut reader)
            .map_err(|err| Either::Left(err))?)
    }

    fn get(&mut self, args: Vec<&str>) -> Result<(), Either<FtpError, io::Error>> {
        let (local_file, remote_file) = (args[1], args[2]);
        Ok(self
            .ftp_stream
            .retr(remote_file, |reader| {
                let mut writer = BufWriter::new(File::create(local_file).unwrap());
                let mut buf = Vec::<u8>::new();
                reader.read_to_end(&mut buf).unwrap();
                writer.write_all(&mut buf).unwrap();
                Ok(())
            })
            .map_err(|err| Either::Left(err))?)
    }
}
fn main() -> Result<(), ftp::FtpError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        panic!("Usage:    ./ftp-client <username> <password> <server_addr> <port>");
    }
    let (username, password, server_addr, port) = (&args[1], &args[2], &args[3], &args[4]);
    let mut ftp_stream = FtpStream::connect(format!("{}:{}", server_addr, port))?;
    ftp_stream.login(username, password)?;
    let stdin = io::stdin();
    let mut ftp_client = FtpClient { ftp_stream };
    loop {
        if let Err(err) = ftp_client.process_command(&stdin) {
            eprintln!("{:?}", err);
            if let Either::Right(err) = err {
                if err.kind() == io::ErrorKind::Interrupted {
                    break Ok(());
                }
            }
        }
    }
}
