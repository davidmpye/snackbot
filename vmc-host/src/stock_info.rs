use std::path::PathBuf;
pub struct StockItem {
    pub row: char,
    pub col: char,
    pub name: String,
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
                        price: 95,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Chilli Peanuts"),
                        price: 125,
                    })
                },
                '4' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Crinklies"),
                        price: 125,
                    })
                },
                '6' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Crinklies"),
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
                        price: 115,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Chilli Doritos"),
                        price: 115,
                    })
                },
                '4' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("McCoy Salt+Vinegar"),
                        price: 125,
                    })
                },
                '6' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("McCoy Sizzling Prawn"),
                        price: 125,
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
                        name: String::from("Star Bar"),
                        price: 75,
                    })
                },
               '1' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Lion Bar"),
                        price: 115,
                    })
                },
               '2' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("M&amp;M Peanut"),
                        price: 100,
                    })
                },
                '3' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Reeses Cups"),
                        price: 119,
                    })
                },
               '4' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Reeses\nNutrageous"),
                        price: 139,
                    })
                },
                '5' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Maltesers"),
                        price: 75,
                    })
                },
                '6' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Snickers"),
                        price: 75,
                    })
                },
                '7' => {
                    Some(StockItem { 
                        row,
                        col,
                        name: String::from("Crunchie"),
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
                        price: 90,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Dr Pepper"),
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
                        price: 90,
                    })
                },
                '1' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Fanta Zero"),
                        price: 90,
                    })
                },
                '2' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("Irn Bru"),
                        price: 90,
                    })
                },
                '3' => {
                    Some(StockItem { 
                        row,col,
                        name: String::from("7UP! Free"),
                        price: 90,
                    })
                },
                _ => None
            }
        }
        _ => None
    }
}
