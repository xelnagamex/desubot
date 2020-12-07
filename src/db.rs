use crate::errors;
use crate::utils;
use rusqlite::{params, Connection, Error, Result};
use std::time::SystemTime;
use telegram_bot::*;

#[derive(Debug, Clone)]
pub struct Conf {
    id: telegram_bot::ChatId,
    title: String,
    date: i32,
}

pub(crate) fn open() -> Result<Connection> {
    let path = "./memory.sqlite3";
    let db = Connection::open(&path)?;
    Ok(db)
}

pub(crate) fn update_scheme() -> Result<()> {
    let conn = open()?;
    conn.execute(SCHEME, params![])?;
    info!("Scheme updated.");
    Ok(())
}

pub(crate) fn get_user(id: telegram_bot::UserId) -> Result<telegram_bot::User, errors::Error> {
    let conn = open()?;
    let mut stmt =
        conn.prepare("SELECT id, username, first_name, last_name, date FROM user WHERE id = :id")?;

    let mut rows = stmt.query_named(&[(":id", &id.to_string())])?;
    let mut users = Vec::new();

    while let Some(row) = rows.next()? {
        users.push(telegram_bot::User {
            id: UserId::new(row.get(0)?),
            username: row.get(1)?,
            is_bot: false,
            first_name: row.get(2)?,
            last_name: row.get(3)?,
            language_code: None,
        })
    }

    if users.len() == 0 {
        Err(errors::Error::UserNotFound)
    } else {
        Ok(users[0].clone())
    }
}

pub(crate) fn get_conf(id: telegram_bot::ChatId) -> Result<Conf, errors::Error> {
    let conn = open()?;
    let mut stmt = conn.prepare("SELECT id, title, date FROM conf WHERE id = :id")?;

    let mut rows = stmt.query_named(&[(":id", &id.to_string())])?;
    let mut confs = Vec::new();
    while let Some(row) = rows.next()? {
        confs.push(Conf {
            id: telegram_bot::ChatId::new(row.get(0)?),
            title: row.get(1)?,
            date: row.get(2)?,
        })
    }
    if confs.len() == 0 {
        Err(errors::Error::ConfNotFound)
    } else {
        Ok(confs[0].clone())
    }
}

/*
pub(crate) fn get_confs() -> Result<Vec<Conf>> {
    let conn = open()?;
    let mut stmt = conn.prepare("SELECT id, title, date FROM conf")?;

    let mut rows = stmt.query(params![])?;
    let mut confs = Vec::new();

    while let Some(row) = rows.next()? {
        confs.push(Conf {
            id: telegram_bot::ChatId::new(row.get(0)?),
            title: row.get(1)?,
            date: row.get(2)?,
        })
    }
    //println!("Confs: {:?}", confs);

    Ok(confs)
}
 */

pub(crate) fn get_members(id: telegram_bot::ChatId) -> Result<Vec<telegram_bot::User>> {
    let conn = open()?;
    let mut stmt = conn.prepare(
        "
        SELECT DISTINCT(u.username), u.id, u.first_name, u.last_name, u.date
        FROM relations r
        JOIN user u
        ON u.id = r.user_id
        LEFT JOIN conf c
        ON r.conf_id = c.id
        WHERE c.id = :id",
    )?;
    let mut rows = stmt.query_named(&[(":id", &id.to_string())])?;
    let mut users = Vec::new();

    while let Some(row) = rows.next()? {
        users.push(telegram_bot::User {
            id: UserId::new(row.get(1)?),
            username: row.get(0)?,
            is_bot: false,
            first_name: row.get(2)?,
            last_name: row.get(3)?,
            language_code: None,
        })
    }
    Ok(users)
}

pub(crate) async fn add_conf(message: Message) -> Result<(), Error> {
    let conn = open()?;
    let title = utils::get_title(&message);

    match get_conf(message.chat.id()) {
        Ok(_) => {
            let update = Conf {
                id: message.chat.id(),
                title,
                date: 0,
            };
            let mut stmt = conn.prepare(
                "UPDATE conf
                SET
                    title = :title
                WHERE
                id = :id",
            )?;
            stmt.execute_named(&[(":id", &update.id.to_string()), (":title", &update.title)])?;
            //println!("Conf {:?} updated: {:?}", update.title, get_conf(update.id));
        }
        Err(_) => {
            let update = Conf {
                id: message.chat.id(),
                title,
                date: 0,
            };
            let unix_time = utils::unixtime().await;

            let mut stmt = conn.prepare(
                "UPDATE conf
                SET
                    title = :title,
                    date = :date
                WHERE
                id = :id",
            )?;
            stmt.execute_named(&[
                (":id", &update.id.to_string()),
                (":title", &update.title),
                (":date", &unix_time),
            ])?;
            //println!("Conf {:?} added: {:?}", update.title, get_conf(update.id));
        }
    }
    Ok(())
}

pub(crate) async fn add_user(message: Message) -> Result<(), Error> {
    let conn = open()?;
    match get_user(message.from.id) {
        Ok(_) => {
            let update = telegram_bot::User {
                id: message.from.id,
                first_name: message.from.first_name,
                last_name: message.from.last_name,
                username: message.from.username,
                is_bot: false,
                language_code: None,
            };
            let mut stmt = conn.prepare(
                "UPDATE user
                SET
                    username = :username,
                    first_name = :first_name,
                    last_name = :last_name
                WHERE
                id = :id",
            )?;
            stmt.execute_named(&[
                (":id", &update.id.to_string()),
                (":username", &update.username),
                (":first_name", &update.first_name),
                (":last_name", &update.last_name),
            ])?;
            //println!("User {} updated: {:?}", update.first_name, get_user(user.id));
        }
        Err(_) => {
            let unix_time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let user = telegram_bot::User {
                id: message.from.id,
                first_name: message.from.first_name,
                last_name: message.from.last_name,
                username: message.from.username,
                is_bot: false,
                language_code: None,
            };
            let mut stmt = conn.prepare(
                "INSERT OR IGNORE INTO
                user('id', 'username', 'first_name', 'last_name', 'date')
                VALUES (:id, :username, :first_name, :last_name, :date)",
            )?;
            stmt.execute_named(&[
                (":id", &user.id.to_string()),
                (":username", &user.username),
                (":first_name", &user.first_name),
                (":last_name", &user.last_name),
                (":date", &unix_time),
            ])?;
            //println!("User added: {:?}", user);
        }
    }
    Ok(())
}

pub(crate) async fn add_file(
    message: &Message,
    path: String,
    file_id: String,
) -> Result<(), Error> {
    let conn = open()?;
    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO
                file('path', 'user_id', 'conf_id', 'file_id')
                VALUES (:path, :user_id, :conf_id, :file_id)",
    )?;
    stmt.execute_named(&[
        (":path", &path),
        (":user_id", &message.from.id.to_string()),
        (":conf_id", &message.chat.id().to_string()),
        (":file_id", &file_id),
    ])?;
    Ok(())
}

pub(crate) async fn get_file(file_id: String) -> Result<i64, errors::Error> {
    let conn = open()?;
    let file_rowid = match { conn.prepare("SELECT rowid FROM file WHERE file_id = :file_id")? }
        .query_row(params![file_id], |row| row.get(0)) {
        Ok(id) => Ok(id),
        Err(_) => {
            Err(errors::Error::FileNotFound)
        }
    };

    file_rowid
}

async fn add_word(word: &String) -> Result<i64, errors::Error> {
    match get_stop_word(&word).await {
        Err(_) => return Err(errors::Error::WordInStopList),
        _ => {}
    }
    let conn = open()?;
    let word_rowid = match { conn.prepare("INSERT OR IGNORE INTO word('word') VALUES (:word)")? }
        .insert(params![word])
    {
        Ok(id) => id,
        Err(_) => { conn.prepare("SELECT rowid FROM word WHERE word = (:word)")? }
            .query_row(params![word], |row| row.get(0))?,
    };
    Ok(word_rowid)
}

async fn get_stop_word(stop_word: &String) -> Result<(), errors::Error> {
    let conn = open()?;
    match conn.execute_named(
        "SELECT rowid FROM stop_words WHERE word = (:stop_word)",
        &[(":stop_word", &stop_word)],
    ) {
        Ok(i) => match i {
            0 => Ok(()),
            _ => Err(errors::Error::WordNotFound),
        },
        Err(e) => Err(errors::Error::SQLITE3Error(e)),
    }
}

async fn add_relation(word_id: i64, msg_id: i64, message: &Message) -> Result<i64, errors::Error> {
    let user_id = i64::from(message.from.id);
    let conf_id = i64::from(message.chat.id());
    let unix_time = utils::unixtime().await;
    let conn = open()?;
    let rowid = match {
        conn.prepare(
            "INSERT OR IGNORE INTO
        relations('word_id', 'user_id', 'conf_id', 'msg_id', 'date')
        VALUES (:word_id, :user_id, :conf_id, :msg_id, :date)",
        )?
    }
    .insert(params![word_id, user_id, conf_id, msg_id, unix_time])
    {
        Ok(id) => id,
        Err(e) => return Err(errors::Error::SQLITE3Error(e)),
    };
    Ok(rowid)
}

pub(crate) async fn add_sentence(message: &telegram_bot::Message) -> Result<(), errors::Error> {
    let text = message.text().unwrap();
    let conn = open()?;

    // Save sentence
    let msg_rowid = match { conn.prepare("INSERT OR IGNORE INTO messages('text') VALUES (:text)")? }
        .insert(params![text])
    {
        Ok(id) => id,
        Err(_) => { conn.prepare("SELECT rowid FROM messages WHERE text = (:text)")? }
            .query_row(params![text], |row| row.get(0))?,
    };

    // Save stemmed words
    let words = utils::stemming(message).await?;
    for word in words {
        match add_word(&word).await {
            Ok(id) => {
                debug!("Added {}: rowid: {}", &word, id);
                match add_relation(id, msg_rowid, message).await {
                    Ok(_) => {},
                    Err(e) => panic!("SQLITE3 Error: Relations failed: {:?}", e)
                }
            }
            Err(_) => debug!("Word {} is in stop list.", &word),
        }
    }

    Ok(())
}

// SCHEME
static SCHEME: &str = "
PRAGMA foreign_keys = off;
BEGIN TRANSACTION;

-- Table: alert
CREATE TABLE IF NOT EXISTS alert (
                       conf_id TEXT NOT NULL,
                       user_id TEXT NOT NULL,
                       created TEXT NOT NULL,
                       time    TEXT NOT NULL,
                       message TEXT
);

-- Table: conf
CREATE TABLE IF NOT EXISTS conf (
                      id    NUMERIC NOT NULL
                          UNIQUE,
                      title TEXT,
                      date  INTEGER NOT NULL,
                      PRIMARY KEY (
                                   id
                          )
);

-- Table: file
CREATE TABLE IF NOT EXISTS file (
                      path    TEXT   NOT NULL,
                      user_id TEXT   NOT NULL,
                      conf_id TEXT   NOT NULL,
                      file_id STRING PRIMARY KEY
);

-- Table: relations
CREATE TABLE IF NOT EXISTS relations (
                           id      INTEGER NOT NULL
                               PRIMARY KEY AUTOINCREMENT,
                           word_id INTEGER NOT NULL,
                           user_id INTEGER NOT NULL,
                           conf_id INTEGER NOT NULL,
                           date    INTEGER NOT NULL,
                           msg_id  INTEGER,
                           FOREIGN KEY (
                                        word_id
                               )
                               REFERENCES word (id) ON DELETE CASCADE,
                           FOREIGN KEY (
                                        user_id
                               )
                               REFERENCES user (id),
                           FOREIGN KEY (
                                        conf_id
                               )
                               REFERENCES conf (id)
);

-- Table: reset
CREATE TABLE IF NOT EXISTS reset (
                       id          INTEGER PRIMARY KEY AUTOINCREMENT,
                       user_id     INTEGER,
                       conf_id     INTEGER,
                       date        INTEGER,
                       relation_id INTEGER,
                       FOREIGN KEY (
                                    user_id
                           )
                           REFERENCES user (id)
);

-- Table: user
CREATE TABLE IF NOT EXISTS user (
                      id         INTEGER NOT NULL
                          UNIQUE,
                      username   TEXT,
                      first_name INTEGER NOT NULL,
                      last_name  INTEGER,
                      date       INTEGER NOT NULL,
                      PRIMARY KEY (
                                   id
                          )
                          ON CONFLICT REPLACE
);

-- Table: word
CREATE TABLE IF NOT EXISTS word (
                      id   INTEGER PRIMARY KEY AUTOINCREMENT,
                      word TEXT    UNIQUE
);

-- Table: xxx_message
CREATE TABLE IF NOT EXISTS xxx_message (
                             id   INTEGER NOT NULL
                                 PRIMARY KEY AUTOINCREMENT,
                             text TEXT    UNIQUE
);

COMMIT TRANSACTION;
PRAGMA foreign_keys = on;
";
