use std::{io::Read, time::Duration};

use tokio::time::interval;
use test_client::{client::WorkbookClient, read_line};

use vmc_icd::{DispenserAddress};
use std::io::{stdout, stdin, Write};
#[tokio::main]
pub async fn main() {

    let client = WorkbookClient::new();

    tokio::select! {
        _ = client.wait_closed() => {
            println!("Client is closed, exiting...");
        }
        _ = run(&client) => {
            println!("App is done")
        }
    }
}

async fn run(client: &WorkbookClient) {



    let mut buf:[u8;64] = [0x00;64];

    let mut got_row = false;
    let mut got_col = false;

    let mut row:char = 'A';
    let mut col: char = '0';

    loop {
            
        let bytes = stdin().read(&mut buf);
        if let Ok(count) = bytes {
            
            for c in &buf[0..count] {
                if c.is_ascii_uppercase() {
                    row = *c as char;
                    got_row = true;
                } 
                else if c.is_ascii_digit() {
                    col = *c as char;
                    got_col = true;
                }

                if got_row && got_col {

                    println!("DOING IT");
                    let item = DispenserAddress { row: row as char, col: col as char};
                    let res = client.dispense(item).await.unwrap();

                    got_row = false;
                    got_col = false;
                }
            }
        }
    }

    loop {
        print!("Please enter command: (VEND)");
        let l = read_line().await;
        let parts :Vec<_>= l.split_ascii_whitespace().collect();

        match parts[0] {
            "VEND" => {
                if parts.len() == 2 {
                    let address = parts[1].as_bytes();
                    if address.len() == 2 {
                        let item = DispenserAddress { row: address[0] as char, col: address[1] as char};
                        let res = client.dispense(item).await.unwrap();
                        if res.is_ok() {
                            print!("Vend successful");
                        }
                        else {
                            println!("Vend failed: {:?}", res.err());
                        }
                    }
                    else {
                        println!("Address format should be 2 characters eg A0");
                    }
                }
                else {
                    println!("No address specified");
                }
            }, 
            "FORCE" => {
                if parts.len() == 2 {
                    let address = parts[1].as_bytes();
                    if address.len() == 2 {
                        let item = DispenserAddress { row: address[0] as char, col: address[1] as char};
                        let res = client.force_dispense(item).await.unwrap();
                        if res.is_ok() {
                            print!("Vend successful");
                        }
                        else {
                            println!("Vend failed: {:?}", res.err());
                        }
                    }
                    else {
                        println!("Address format should be 2 characters eg A0");
                    }
                }
                else {
                    println!("No address specified");
                }
            }
            "STATUS" => {
                if parts.len() == 2 {
                    let address = parts[1].as_bytes();
                    if address.len() == 2 {
                        let item = DispenserAddress { row: address[0] as char, col: address[1] as char};
                        let res = client.dispenser_status(item).await.unwrap();
                        match res {
                            Some(x) => println!("{:?}", x),
                            _=> println!("Err"),
                        }
                    }
                    else {
                        println!("Address format should be 2 characters eg A0");
                    }
                }
                else {
                    println!("No address specified");
                }

            },
            "MAP" => {
                //prod all possible addresses
                for row in 'A'..'G' {
                    for col in '0'..'9' {
                        let disp = client.dispenser_status(DispenserAddress {row: row, col: col}).await;
                        match disp.unwrap() {
                            Some (d) => println!("{:?}",d),
                            _ => {},
                        }
                    }
                }
            }
            _ => {},
        };
    
}

}
