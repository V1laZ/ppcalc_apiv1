use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use std::io::prelude::*;

#[derive(Deserialize)]
struct UserBestResponse {
    pp: String,
}

#[derive(Serialize, Deserialize, Default)]
struct MyConfig {
    api_key: String,
}

lazy_static! {
    static ref CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::new();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg: MyConfig = confy::load("ppcalc_apiv1", "creds-config")?;

    if cfg.api_key.is_empty() {
        println!("Please enter your osu! API key:");
        let mut api_key = String::new();
        std::io::stdin().read_line(&mut api_key)?;
        cfg.api_key = api_key.trim().to_string();

        confy::store("ppcalc_apiv1", "creds-config", &cfg)?;
    }

    println!("Enter user:");
    let mut user = String::new();
    std::io::stdin().read_line(&mut user)?;
    let user = user.trim();

    println!("Fetching data...");
    let user_best = get_user_best(user, &cfg.api_key)?;

    println!("Select action:");
    println!("1. Calculate new play");
    println!("2. Calculate needed play");
    let mut action = String::new();
    std::io::stdin().read_line(&mut action)?;
    let action = action.trim();

    match action {
        "1" => {
            println!("Enter new play:");
            let mut new_play = String::new();
            std::io::stdin().read_line(&mut new_play)?;
            let new_play: f64 = new_play.trim().parse()?;

            println!("Calculating new play...");
            let added_pp = calc_new_play(&user_best, new_play);

            println!("The {}pp play will gain you {:.2}pp", new_play, added_pp);
            pause();
            return Ok(());
        },
        "2" => {
            println!("Enter target pp:");
            let mut target_pp = String::new();
            std::io::stdin().read_line(&mut target_pp)?;
            let target_pp: f64 = target_pp.trim().parse()?;

            println!("Calculating needed play...");
            let needed_play = calc_needed_play(&user_best, target_pp);

            println!("You need to make a {:.2}pp play in order to gain {}pp", needed_play, target_pp);
            pause();
            return Ok(());
        },
        _ => {
            println!("Invalid action selected.");
            pause();
            return Ok(());
        }
    }
}

fn get_user_best(user: &str, api_key: &str) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let res = CLIENT.get("https://osu.ppy.sh/api/get_user_best")
        .header("Accept", "application/json")
        .query(&[("mode", "osu"), ("limit", "100"), ("k", api_key), ("u", user)])
        .send()?;

    let body: Vec<UserBestResponse> = res.json()?;
    let mut pp_list: Vec<f64> = Vec::new();
    for play in body {
        pp_list.push(play.pp.parse()?);
    }

    Ok(pp_list)
}

fn recalc_to_weighted(users_best: &Vec<f64>) -> Vec<f64> {
    let mut weighted_pp_list: Vec<f64> = Vec::new();

    for (i, pp) in users_best.iter().enumerate() {
        let weighted = pp * 0.95_f64.powf(i as f64);
        weighted_pp_list.push(weighted);
    }

    weighted_pp_list
}

fn calc_new_play(users_best: &Vec<f64>, new_play: f64) -> f64 {
    let last_play = users_best.last().unwrap_or(&0.0);
    if new_play < *last_play {
        return 0.0;
    }

    let weighted_before = recalc_to_weighted(&users_best).iter().sum::<f64>();

    let mut new_users_best = users_best.clone();
    new_users_best.push(new_play);
    new_users_best.sort_by(|a, b| b.partial_cmp(a).unwrap());
    new_users_best.pop();

    let weighted_after = recalc_to_weighted(&new_users_best).iter().sum::<f64>();

    weighted_after - weighted_before
}

fn calc_needed_play(users_best: &Vec<f64>, target_pp: f64) -> f64 {
    let weighted_before = recalc_to_weighted(&users_best).iter().sum::<f64>();
    let mut users_best_copy = users_best.clone();

    let users_best_len = users_best_copy.len();
    let mut pre: Vec<f64> = Vec::new();
    let mut post: Vec<f64> = Vec::new();
    
    users_best_copy.pop();

    pre.push(0.0);
    for (i, play) in users_best_copy.iter().enumerate() {
        pre.push(play * 0.95_f64.powf(i as f64) + pre[i]);
    };

    post.push(0.0);
    for (i, play) in users_best_copy.iter().enumerate().rev() {
        post.push(play * 0.95_f64.powf(i as f64) + post[post.len() - 1]);
    };

    post.reverse();

    let new_target = weighted_before + target_pp;

    let mut needs_play = 0.0;
    for i in 0..users_best_len {
        let num = (new_target - pre[i] - 0.95 * post[i]) / (0.95_f64.powf(i as f64));
        if (i == 0 || num <= users_best_copy[i]) && (i == users_best_len || num >= users_best_copy[i + 1]) {
            needs_play = num;
            break;
        }
    }

    needs_play
}

fn pause() {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    write!(stdout, "Press Enter to exit...").unwrap();
    stdout.flush().unwrap();
    stdin.read(&mut [0]).unwrap();
}