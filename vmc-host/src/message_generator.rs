use rand::RngExt;

pub struct MessageGenerator {
    welcome_messages: Vec<&'static str>,
    thankyou_messages: Vec<&'static str>,
}

impl MessageGenerator {
    pub fn new() -> Self {
        Self {
            welcome_messages :vec![
                "sudo buy snack",
                "404: Snack missing
                Please insert GBP",
                "Buy snacks. Build stuff.",
                "One does not simply
                walk past snacks.",
                "Your lack of snacks
                disturbs me.",
                "Resistance is futile.
                Insert GBP.",
                "These aren't the
                snacks you're after.",
                "Do. Or do not.
                Buy snacks. There is no try."    
            ],
            thankyou_messages : vec![
                "Thank you.
                Live long and prosper.",
                "Make it so.
                Transaction complete.",
                "Transaction complete.
                Achievement unlocked!",
                "+10 XP
                +1 Snack",
                "Much appreciated.
                Very snack. Wow.",
                "200: Snack found",
                "Server fund +1",
                "Snack. Hack. Repeat.",
            ],
        }
    }   

    pub fn get_welcome_message(&self) ->  (&'static str, &'static str) {
        let random_index = rand::rng().random_range(0..self.welcome_messages.len());
        reformat_message(self.welcome_messages[random_index])
    }

    pub fn get_thankyou_message(&self) -> (&'static str, &'static str) {
        let random_index = rand::rng().random_range(0..self.thankyou_messages.len());
        reformat_message(self.thankyou_messages[random_index])
    }

}


fn reformat_message(msg: &str)  -> (&str, &str) {
    if let Some((first, second)) = msg.split_once('\n') {
        (first.trim(), second.trim())
    }
    else {
        (msg, "")
    }
}
