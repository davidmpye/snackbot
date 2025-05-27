use std::path::PathBuf;
pub struct StockItem {
    pub row: char,
    pub col: char,
    pub name: String,
    pub image_url: String,
    pub price: u16,
}

pub fn get_stock_item(row:char, col:char) -> Option<StockItem> {
    match row {
        //Three spirals

        'A' => {
            match col {
                '0' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Scampi Fries"),
                        image_url: String::from("images/scampi.jpg"),
                        price: 90,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Bacon Fries"),
                        image_url: String::from("images/baconfries.jpg"),
                        price: 90,
                    })
                },
                '4' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Crinklies"),
                        image_url: String::from("images/crinklies.jpg"),
                        price: 100,
                    })
                },
                '6' => {
                    Some(StockItem { 
                         row,
                        col,
                        name: String::from("Monster Munch"),
                        image_url: String::from("images/monstermunch.jpg"),
                        price: 100,
                    })
                },
                _ => None,
            }
        }
        'B' => {
            match col {
                '0' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Tangy Cheese Doritos"),
                        image_url: String::from("images/tangycheesedoritos.jpg"),
                        price: 100,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Chilli Doritos"),
                        image_url: String::from("images/chilliheatwavedoritos.jpg"),
                        price: 100,
                    })
                },
                '4' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Soba Noodles"),
                        image_url: String::from("./doritos.jpg"),
                        price: 150,
                    })
                },
                '6' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Super Noodles"),
                        image_url: String::from("./doritos.jpg"),
                        price: 130,
                    })
                },
                _ => None,
            }
        }
        'C' => {
            match col {
                '3' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Reeses\nCups"),
                        image_url: String::from("images/chilliheatwavedoritos.jpg"),
                        price: 100,
                    })
                },
                _ => None,
            }
        }
        //Cans
        'E' => {
            match col {
                '0' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Cream Soda"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                '1' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Super Noodles"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Super Noodles"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                '3' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Super Noodles"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                _ => None
            }
        }
        'F' => {
            match col {
                '0' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Diet Coke"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                '1' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Fanta Zero"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Irn Bru"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                '3' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("7UP! Free"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                _ => None
            }
        }
        _ => None
    }
}