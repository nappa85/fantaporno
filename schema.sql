CREATE TABLE pornstars (id INTEGER PRIMARY KEY AUTOINCREMENT, name VARCHAR(255), url VARCHAR(255));
CREATE TABLE positions (pornstar_id INTEGER, date DATETIME, position INTEGER, PRIMARY KEY (pornstar_id, date));
