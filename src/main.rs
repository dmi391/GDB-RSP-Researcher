pub mod sim;
use sim::*;
pub mod gdb_server;
use gdb_server::*;


fn main()
{
    parse_args();
    gdb_server();

    println!("End of execution!");
}
