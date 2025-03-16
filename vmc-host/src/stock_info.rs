use std::path::PathBuf;
use crate::DispenserAddress;
pub struct StockItem {
    pub address: DispenserAddress,
    pub name: String,
    pub image_url: String,
    pub price: u16,
}

pub fn get_stock_item(address: DispenserAddress) -> Option<StockItem> {
    match address {
        DispenserAddress{row:'A',col:'0'} => {
            Some(StockItem { 
                address,
                name: String::from("Scampi Fries"),
                image_url: String::from("./scampi.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'B',col:'0'} => {
            Some(StockItem { 
                address,
                name: String::from("Chilli Doritos"),
                image_url: String::from("./doritos.jpg"),
                price: 100,
            })
        },
        _ => None,
    }

}