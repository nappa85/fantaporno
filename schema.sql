CREATE TABLE pornstars (id INTEGER PRIMARY KEY AUTOINCREMENT, name VARCHAR(255), url VARCHAR(255));
CREATE TABLE positions (pornstar_id INTEGER, date DATETIME, position INTEGER, PRIMARY KEY (pornstar_id, date));
CREATE TABLE players (telegram_id UNSIGNED INTEGER PRIMARY KEY, name VARCHAR(255), budget UNSIGNED INTEGER);
CREATE TABLE teams (player_id UNSIGNED INTEGER, pornstar_id INTEGER, start_date DATETIME, end_date DATETIME DEFAULT NULL);
