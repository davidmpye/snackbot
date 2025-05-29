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
                        image_url: String::from("images/scampi_fries.jpg"),
                        price: 95,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Chilli Peanuts"),
                        image_url: String::from("images/chilli_peanuts.jpg"),
                        price: 125,
                    })
                },
                '4' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Crinklies"),
                        image_url: String::from("images/crinklies.jpg"),
                        price: 125,
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
                        image_url: String::from("images/doritos_cheese.jpg"),
                        price: 115,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Chilli Doritos"),
                        image_url: String::from("images/doritos_heatwave.jpg"),
                        price: 115,
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
                '0' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Kitkat\nPeanut Butter"),
                        image_url: String::from("images/tangycheesedoritos.jpg"),
                        price: 75,
                    })
                },
               '1' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("EatNatural\nBar"),
                        image_url: String::from("images/tangycheesedoritos.jpg"),
                        price: 115,
                    })
                },
               '2' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("M&amp;M Peanut"),
                        image_url: String::from("images/tangycheesedoritos.jpg"),
                        price: 100,
                    })
                },
                '3' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Reeses Cups"),
                        image_url: String::from("images/peanut_butter_cups.jpg"),
                        price: 119,
                    })
                },
               '4' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Reeses\nNutrageous"),
                        image_url: String::from("images/nutrageous.jpg"),
                        price: 139,
                    })
                },
                '5' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Maltesers"),
                        image_url: String::from("images/tangycheesedoritos.jpg"),
                        price: 75,
                    })
                },
                '6' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Snickers"),
                        image_url: String::from("images/tangycheesedoritos.jpg"),
                        price: 75,
                    })
                },
                '7' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Crunchie"),
                        image_url: String::from("images/tangycheesedoritos.jpg"),
                        price: 89,
                    })
                },

                _ => None,
            }
        }
        //Cans
        'E' => {
            match col {
                '1' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Cream Soda"),
                        image_url: String::from("./doritos.jpg"),
                        price: 90,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Dr Pepper"),
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