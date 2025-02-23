use std::path::PathBuf;
use crate::DispenserAddress;
pub struct StockItem {
    address: DispenserAddress,
    name: String,
    image_url: PathBuf,
}

pub fn get_stock_item(address: DispenserAddress) -> Option<StockItem> {
    match address {
        DispenserAddress{row:'A',col:'0'} => {
            Some(StockItem { 
                address,
                name: String::from("Scampi Fries"),
                image_url: PathBuf::from("./scampi.jpg"),
            })
        },
        DispenserAddress{row:'B',col:'0'} => {
            Some(StockItem { 
                address,
                name: String::from("Chilli Doritos"),
                image_url: PathBuf::from("./doritos.jpg"),
            })
        },
        _ => None,
    }

}