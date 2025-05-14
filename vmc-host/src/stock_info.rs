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
                image_url: String::from("images/scampi.jpg"),
                price: 90,
            })
        },
        DispenserAddress{row:'A',col:'2'} => {
            Some(StockItem { 
                address,
                name: String::from("Bacon Fries"),
                image_url: String::from("images/baconfries.jpg"),
                price: 90,
            })
        },
        DispenserAddress{row:'A',col:'4'} => {
            Some(StockItem { 
                address,
                name: String::from("Crinklies"),
                image_url: String::from("images/crinklies.jpg"),
                price: 100,
            })
        },        
        DispenserAddress{row:'A',col:'6'} => {
            Some(StockItem { 
                address,
                name: String::from("Monster Munch"),
                image_url: String::from("images/monstermunch.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'B',col:'0'} => {
            Some(StockItem { 
                address,
                name: String::from("Tangy Cheese Doritos"),
                image_url: String::from("images/tangycheesedoritos.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'B',col:'2'} => {
            Some(StockItem { 
                address,
                name: String::from("Chilli Doritos"),
                image_url: String::from("images/chilliheatwavedoritos.jpg"),
                price: 100,
            })
        },        
        DispenserAddress{row:'B',col:'4'} => {
            Some(StockItem { 
                address,
                name: String::from("Soba Noodles"),
                image_url: String::from("./doritos.jpg"),
                price: 150,
            })
        },        
        DispenserAddress{row:'B',col:'6'} => {
            Some(StockItem { 
                address,
                name: String::from("Super Noodles"),
                image_url: String::from("./doritos.jpg"),
                price: 130,
            })
        },
        DispenserAddress{row:'C',col:'0'} => {
            Some(StockItem { 
                address,
                name: String::from("Nature Valley Bar"),
                image_url: String::from("./doritos.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'C',col:'1'} => {
            Some(StockItem { 
                address,
                name: String::from("Crunchie"),
                image_url: String::from("./doritos.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'C',col:'2'} => {
            Some(StockItem { 
                address,
                name: String::from("Cadbury's Snack"),
                image_url: String::from("./doritos.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'C',col:'3'} => {
            Some(StockItem { 
                address,
                name: String::from("Reese's Nutrageous"),
                image_url: String::from("./doritos.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'C',col:'4'} => {
            Some(StockItem { 
                address,
                name: String::from("Reeses' Peanut Butter Cups"),
                image_url: String::from("./doritos.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'C',col:'6'} => {
            Some(StockItem { 
                address,
                name: String::from("M&Ms"),
                image_url: String::from("./doritos.jpg"),
                price: 100,
            })
        },
        DispenserAddress{row:'C',col:'7'} => {
            Some(StockItem { 
                address,
                name: String::from("Lion Bar"),
                image_url: String::from("./doritos.jpg"),
                price: 100,
            })
        },

        

        DispenserAddress{row:'E',col:'1'} => {
            Some(StockItem { 
                address,
                name: String::from("Cream Soda"),
                image_url: String::from("./doritos.jpg"),
                price: 90,
            })
        },

        DispenserAddress{row:'E',col:'2'} => {
            Some(StockItem { 
                address,
                name: String::from("Doctor Pepper"),
                image_url: String::from("./doritos.jpg"),
                price: 90,
            })
        },
        DispenserAddress{row:'E',col:'3'} => {
            Some(StockItem { 
                address,
                name: String::from("Diet Coke"),
                image_url: String::from("./doritos.jpg"),
                price: 90,
            })
        },
        DispenserAddress{row:'F',col:'0'} => {
            Some(StockItem { 
                address,
                name: String::from("Diet Coke"),
                image_url: String::from("./doritos.jpg"),
                price: 90,
            })
        },
        DispenserAddress{row:'F',col:'1'} => {
            Some(StockItem { 
                address,
                name: String::from("Fanta Sugar Free"),
                image_url: String::from("./doritos.jpg"),
                price: 90,
            })
        },
        DispenserAddress{row:'F',col:'2'} => {
            Some(StockItem { 
                address,
                name: String::from("Irn Bru Sugar Free"),
                image_url: String::from("./doritos.jpg"),
                price: 90,
            })
        },
        DispenserAddress{row:'F',col:'3'} => {
            Some(StockItem { 
                address,
                name: String::from("7UP Sugar Free"),
                image_url: String::from("./doritos.jpg"),
                price: 90,
            })
        },


        _ => None,
    }

}