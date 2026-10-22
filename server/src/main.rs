use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("Hello from server!");

    Ok(())
}
