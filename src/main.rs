// #![feature(custom_attribute)]
extern crate chrono;
extern crate futures;
extern crate telegram_bot;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate serde_json;

mod models;
mod parsing;
mod schema;

use models::{Group, User};

use chrono::prelude::*;
use diesel::dsl::*;
use diesel::insert_into;
use diesel::prelude::*;
use dotenv::dotenv;
use futures::StreamExt;
use std::env;
use telegram_bot::prelude::*;
use telegram_bot::types::*;
use telegram_bot::{Api, Message, MessageKind, ParseMode, UpdateKind};

use schema::groups::dsl::*;
use schema::users::dsl::*;

// #[derive(Serialize, Deserialize, Debug)]
// struct Group {
//     group: String,
//     id: String,
//     users: Vec<i64>,
// }

// #[derive(Debug)]
// struct User {
//     id: i64,
//     name: String,
//     group: Group,
// }

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

fn send_message(api: Api, message: Message) {
    if let MessageKind::Text { ref data, .. } = message.kind {
        let connection = establish_connection();

        let group = groups
            .filter(name.eq(data))
            .get_result::<Group>(&connection)
            .unwrap();
        let mut user: User = match select(exists(
            users.filter(tg_id.eq(i64::from(message.from.id) as i32)),
        ))
        .get_result(&connection)
        {
            Ok(true) => users
                .filter(tg_id.eq(i64::from(message.from.id) as i32))
                .get_result::<User>(&connection)
                .unwrap(),
            Ok(false) => {
                insert_into(users)
                    .values(&vec![(
                        tg_id.eq(i64::from(message.from.id) as i32),
                        tg_name.eq(&message.from.first_name),
                        group_id.eq(group.id),
                    )])
                    .execute(&connection)
                    .unwrap();
                users
                    .filter(tg_id.eq(i64::from(message.from.id) as i32))
                    .get_result(&connection)
                    .unwrap()
            }
            Err(err) => panic!("send msg {:?}", err),
        };
        diesel::update(users.find(user.id))
            .set(group_id.eq(group.id))
            .execute(&connection);
        let mut inline_keyboard = InlineKeyboardMarkup::new();
        inline_keyboard.add_row(
            [
                InlineKeyboardButton::callback("На сегодня", format!("day,{},0", &group.api_id)),
                InlineKeyboardButton::callback("На завтра", format!("day,{},1", &group.api_id)),
            ]
            .to_vec(),
        );
        inline_keyboard.add_row(
            [InlineKeyboardButton::callback(
                "Неделя",
                format!("week,{},0", &group.api_id),
            )]
            .to_vec(),
        );
        // if user.notify {
        //     inline_keyboard.add_row(
        //         [InlineKeyboardButton::callback(
        //             "Выкл. уведомления 🔕",
        //             format!("notify,{}", &group.api_id),
        //         )].to_vec(),
        //     );
        // } else {
        //     inline_keyboard.add_row(
        //         [InlineKeyboardButton::callback(
        //             "Вкл. уведомления 🔔 ~20:00",
        //             format!("notify,{}", &group.api_id),
        //         )].to_vec(),
        //     );
        // }
        let mut reply_keyboard = ReplyKeyboardMarkup::new();
        if user.notify {
            reply_keyboard.add_row([KeyboardButton::new("Выкл. уведомления 🔕")].to_vec());
        } else {
            reply_keyboard.add_row([KeyboardButton::new("Вкл. уведомления 🔔")].to_vec());
        }
        reply_keyboard.resize_keyboard();

        api.spawn(message.from.text("Выбирай").reply_markup(reply_keyboard));
        api.spawn(
            message
                .from
                .text(format!("Группа - {}", &group.name))
                .reply_markup(inline_keyboard),
        );
    }
}

fn send_start(api: Api, message: Message) {
    api.spawn(
        message
            .from
            .text("Напиши номер группы.")
            .reply_markup(ReplyKeyboardRemove::new()),
    );
}

fn send_notify(api: Api, message: Message) {
    let mut msg = String::new();
    let connection = establish_connection();

    let mut user: User = match select(exists(
        users.filter(tg_id.eq(i64::from(message.from.id) as i32)),
    ))
    .get_result(&connection)
    {
        Ok(true) => users
            .filter(tg_id.eq(i64::from(message.from.id) as i32))
            .get_result::<User>(&connection)
            .unwrap(),
        Ok(false) => panic!("alarm msg false"),
        Err(err) => panic!("send msg {:?}", err),
    };
    if user.notify {
        user.notify = false;
        msg += "Уведомления выключены.";
    } else {
        user.notify = true;
        msg += "Уведомления включены, в ~20:00 будет расписание на завтра.";
    }
    diesel::update(users.find(user.id))
        .set(notify.eq(user.notify))
        .execute(&connection)
        .unwrap();
    // let groups: &Vec<serde_json::Value> = &notify.as_array_mut().unwrap();

    let mut reply_keyboard = ReplyKeyboardMarkup::new();
    if user.notify {
        reply_keyboard.add_row([KeyboardButton::new("Выкл. уведомления 🔕")].to_vec());
    } else {
        reply_keyboard.add_row([KeyboardButton::new("Вкл. уведомления 🔔")].to_vec());
    }
    reply_keyboard.resize_keyboard();

    // println!("{:?}", Some(&inline_keyboard));
    api.spawn(
        UserId::new(i64::from(user.tg_id))
            .text(msg)
            .reply_markup(reply_keyboard),
    );
}

async fn check_message(api: Api, message: Message) -> Result<(), Error> {
    let connection = establish_connection();

    if let MessageKind::Text { ref data, .. } = message.kind {
        match diesel::dsl::select(diesel::dsl::exists(groups.filter(name.eq(data))))
            .get_result(&connection)
        {
            Ok(true) => send_message(api, message),
            _ => match data.as_str() {
                "/start" => send_start(api, message),
                "Вкл. уведомления 🔔" => send_notify(api, message),
                "Выкл. уведомления 🔕" => send_notify(api, message),
                &_ => (),
            },
        }
    } else {
        ();
    }

    Ok(())
}

async fn check_callback(api: Api, query: CallbackQuery) -> Result<(), Error> {
    let message = query.message;
    println!("{:?}, {:?}", query.data, query.from);
    let mut inline_keyboard = InlineKeyboardMarkup::new();
    let data = query.data.unwrap();
    let data = data.split(',').collect::<Vec<_>>();
    let date = Local::now().weekday().number_from_monday();
    let connection = establish_connection();
    let group = match data.len() {
        1 => groups
            .filter(api_id.eq(data[0]))
            .get_result::<Group>(&connection)
            .unwrap(),
        _ => groups
            .filter(api_id.eq(data[1]))
            .get_result::<Group>(&connection)
            .unwrap(),
    };
    let mut user: User = match select(exists(
        users.filter(tg_id.eq(i64::from(query.from.id) as i32)),
    ))
    .get_result(&connection)
    {
        Ok(true) => users
            .filter(tg_id.eq(i64::from(query.from.id) as i32))
            .get_result::<User>(&connection)
            .unwrap(),
        Ok(false) => {
            insert_into(users)
                .values(&vec![(
                    tg_id.eq(i64::from(query.from.id) as i32),
                    tg_name.eq(query.from.first_name),
                    group_id.eq(group.id),
                )])
                .execute(&connection)
                .unwrap();
            users
                .filter(tg_id.eq(i64::from(query.from.id) as i32))
                .get_result(&connection)
                .unwrap()
        }
        Err(err) => panic!("{:?}", err),
    };

    let days = if data.len() == 3 {
        data[2].parse::<i64>().unwrap_or(0)
    } else {
        0i64
    };
    // TODAY
    if data[0].contains("day") {
        let out = parsing::parse_day(data[1], date + days as u32, 0);
        inline_keyboard
            .add_row([InlineKeyboardButton::callback("←", &data[1].to_string())].to_vec());
        api.send(
            message
                .unwrap()
                .edit_text(out.await)
                .parse_mode(ParseMode::Markdown)
                .reply_markup(inline_keyboard),
        ).await;
    } else if data[0].contains("week-") {
        //WEEK
        // println!("{:?}", query.data);
        let date = data[0].chars().last().unwrap().to_digit(10).unwrap();
        let mut days = if data.len() == 3 {
            data[2].parse::<i64>().unwrap_or(0)
        } else {
            0i64
        };
        let out = parsing::parse_day(data[1], date, days);

        inline_keyboard.add_row(
            [InlineKeyboardButton::callback(
                "←",
                format!("week,{},{}", &data[1], days),
            )]
            .to_vec(),
        );
        api.send(
            message
                .unwrap()
                .edit_text(out.await)
                .parse_mode(ParseMode::Markdown)
                .reply_markup(inline_keyboard),
        )
        .await;
    // let edit_keyboard = api.send(message.edit_reply_markup(Some(inline_keyboard)));
    } else if data[0].contains("week") {
        // PRINT WEEK
        // println!("{:?}", query.data);
        let mut days = if data.len() == 3 {
            data[2].parse::<i64>().unwrap_or(0)
        } else {
            0i64
        };
        inline_keyboard.add_row(
            [
                InlineKeyboardButton::callback("Пн", format!("week-1,{},{}", &data[1], days)),
                InlineKeyboardButton::callback("Вт", format!("week-2,{},{}", &data[1], days)),
                InlineKeyboardButton::callback("Ср", format!("week-3,{},{}", &data[1], days)),
            ]
            .to_vec(),
        );
        inline_keyboard.add_row(
            [
                InlineKeyboardButton::callback("Чт", format!("week-4,{},{}", &data[1], days)),
                InlineKeyboardButton::callback("Пт", format!("week-5,{},{}", &data[1], days)),
                InlineKeyboardButton::callback("Сб", format!("week-6,{},{}", &data[1], days)),
            ]
            .to_vec(),
        );
        inline_keyboard.add_row(
            [
                InlineKeyboardButton::callback("⇠", format!("week,{},{}", &data[1], days - 7)),
                InlineKeyboardButton::callback("⇢", format!("week,{},{}", &data[1], days + 7)),
            ]
            .to_vec(),
        );
        inline_keyboard
            .add_row([InlineKeyboardButton::callback("←", &data[1].to_string())].to_vec());
        let out = parsing::parse_week(data[1], days).await;
        api.send(
            message
                .unwrap()
                .edit_text(out)
                .parse_mode(ParseMode::Markdown)
                .reply_markup(inline_keyboard),
        )
        .await;
    } else if data[0].contains("notify") {
        if user.notify {
            user.notify = false;
        } else {
            user.notify = true
        }
        diesel::update(users.find(user.id))
            .set((notify.eq(user.notify), group_id.eq(group.id)))
            .execute(&connection)
            .unwrap();
        // let groups: &Vec<serde_json::Value> = &notify.as_array_mut().unwrap();

        inline_keyboard.add_row(
            [
                InlineKeyboardButton::callback("На сегодня", format!("day,{},0", &group.api_id)),
                InlineKeyboardButton::callback("На завтра", format!("day,{},1", &data[0])),
            ]
            .to_vec(),
        );
        inline_keyboard.add_row(
            [InlineKeyboardButton::callback(
                "Неделя",
                format!("week,{},0", &group.api_id),
            )]
            .to_vec(),
        );
        // if user.notify {
        //     inline_keyboard.add_row(
        //         [InlineKeyboardButton::callback(
        //             "Выкл уведомления 🔕",
        //             format!("notify,{}", &group.api_id),
        //         )].to_vec(),
        //     );
        // } else {
        //     inline_keyboard.add_row(
        //         [InlineKeyboardButton::callback(
        //             "Вкл. уведомления 🔔 ~20:00",
        //             format!("notify,{}", &group.api_id),
        //         )].to_vec(),
        //     );
        // }

        // println!("{:?}", Some(&inline_keyboard));
        api.send(
            message
                .unwrap()
                .edit_text(format!("Группа - {}", &group.name))
                .reply_markup(inline_keyboard),
        )
        .await;
    } else {
        //GROUP
        // println!("{:?}", &query.data);
        inline_keyboard.add_row(
            [
                InlineKeyboardButton::callback("На сегодня", format!("day,{},0", &group.api_id)),
                InlineKeyboardButton::callback("На завтра", format!("day,{},1", &group.api_id)),
            ]
            .to_vec(),
        );
        inline_keyboard.add_row(
            [InlineKeyboardButton::callback(
                "Неделя",
                format!("week,{},0", &group.api_id),
            )]
            .to_vec(),
        );
        // if user.notify {
        //     inline_keyboard.add_row(
        //         [InlineKeyboardButton::callback(
        //             "Выкл. уведомления 🔕",
        //             format!("notify,{}", &group.api_id),
        //         )].to_vec(),
        //     );
        // } else {
        //     inline_keyboard.add_row(
        //         [InlineKeyboardButton::callback(
        //             "Вкл. уведомления 🔔 ~20:00",
        //             format!("notify,{}", &group.api_id),
        //         )].to_vec(),
        //     );
        // }
        // println!("{:?}", Some(&inline_keyboard));
        api.send(
            message
                .unwrap()
                .edit_text(format!("Группа - {}", &group.name))
                .reply_markup(inline_keyboard),
        )
        .await;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = String::from("512619758:AAEnuB3M8Cb9gmMgsrDkeicu-h9hhgonq54");
    let api = Api::new(token);

    let mut stream = api.stream();

    while let Some(update) = stream.next().await {
        let update = update.unwrap();
        if let UpdateKind::Message(message) = update.kind {
            check_message(api.clone(), message).await?;
        } else if let UpdateKind::CallbackQuery(query) = update.kind {
            check_callback(api.clone(), query).await?;
        }
    }

    // thread::spawn(move || {
    //     let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();

    //     let mut core = Core::new().unwrap();
    //     let api = Api::configure(&token).build(core.handle()).unwrap();
    //     loop {
    //         let connection = establish_connection();
    //         if Local::now().hour() == 20 && Local::now().weekday().number_from_monday() != 6 {
    //             let results = users
    //                 .filter(notify.eq(true))
    //                 .get_results::<User>(&connection)
    //                 .unwrap();

    //             for user in results {
    //                 let group = groups
    //                     .find(user.group_id)
    //                     .get_result::<Group>(&connection)
    //                     .unwrap();
    //                 let mut out = "Пары на завтра:\n".to_string();
    //                 out += &parsing::parse_day(
    //                     &group.api_id,
    //                     Local::now().checked_add_signed(chrono::Duration::days(1)).unwrap().weekday().number_from_monday(),
    //                     0,
    //                 );
    //                 let mut inline_keyboard = InlineKeyboardMarkup::new();
    //                 inline_keyboard.add_row(
    //                     [
    //                         InlineKeyboardButton::callback(
    //                             "На сегодня",
    //                             format!("day,{},0", &group.api_id),
    //                         ),
    //                         InlineKeyboardButton::callback(
    //                             "На завтра",
    //                             format!("day,{},1", &group.api_id),
    //                         ),
    //                     ].to_vec(),
    //                 );
    //                 inline_keyboard.add_row(
    //                     [InlineKeyboardButton::callback(
    //                         "Неделя",
    //                         format!("week,{},0", &group.api_id),
    //                     )].to_vec(),
    //                 );
    //                 // if user.notify {
    //                 //     inline_keyboard.add_row(
    //                 //         [InlineKeyboardButton::callback(
    //                 //             "Выкл. уведомления 🔕",
    //                 //             format!("notify,{}", &group.api_id),
    //                 //         )].to_vec(),
    //                 //     );
    //                 // } else {
    //                 //     inline_keyboard.add_row(
    //                 //         [InlineKeyboardButton::callback(
    //                 //             "Вкл. уведомления 🔔 ~20:00",
    //                 //             format!("notify,{}", &group.api_id),
    //                 //         )].to_vec(),
    //                 //     );
    //                 // }
    //                 core.run(
    //                     api.send(
    //                         UserId::new(i64::from(user.tg_id))
    //                             .text(&out)
    //                             .reply_markup(inline_keyboard)
    //                             .parse_mode(ParseMode::Markdown),
    //                     ),
    //                 ).unwrap();
    //             }
    //         }
    //         thread::sleep(Duration::from_secs(1000*60));
    //     }
    // });

    // handle.join().unwrap();
    Ok(())
}
