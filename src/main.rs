
pub mod gdb_server;
use gdb_server::*;

fn main()
{
    gdb_server();

    println!("End of execution!");
}
