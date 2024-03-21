use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        panic!("Usage:    ./smtp-client <filename> <mail_addr> <login> <password>");
    }
    let (filename, addr_to, login, password) = (&args[1], &args[2], &args[3], &args[4]);
    let file = Path::new(&filename);
    let content_type = match file.extension() {
        Some(str) if str == "html" || str == "txt" => {
            if str == "html" {
                ContentType::TEXT_HTML
            } else {
                ContentType::TEXT_PLAIN
            }
        }
        _ => {
            panic!("File of .txt or .html expected");
        }
    };
    let content = std::fs::read_to_string(file).expect("Cannot read file");
    let email = Message::builder()
        .from(login.parse().unwrap())
        .to(addr_to.parse().unwrap())
        .subject("Test")
        .header(content_type)
        .body(content)
        .unwrap();

    let creds = Credentials::new(login.to_owned(), password.to_owned());

    // Open a remote connection to mail
    let mailer = SmtpTransport::relay("smtp.mail.ru")
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {e:?}"),
    }
}
