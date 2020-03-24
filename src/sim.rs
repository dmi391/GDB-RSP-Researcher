
pub fn parse_args() -> bool
{
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1
    {
        match &args[1][..]
        {
            "--loop" | "-l"=>
            {
                println!("  Started with loop run similation\n");
                true
            },
            _=>
            {
                println!("  Unknown argument {:?}\n", args[1]);
                false
            }
        }
    }
    else
    {
        false
    }
}

pub fn run_sim() -> Result<(),()>
{//Run simulation
    if parse_args() == true
    {
        loop
        {
            //if ...
            //    break
        };
    }
    Ok(())
}
