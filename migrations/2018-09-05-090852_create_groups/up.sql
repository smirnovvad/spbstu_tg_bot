-- Your SQL goes here
CREATE TABLE groups (
  id INTEGER PRIMARY KEY NOT NULL,
  name VARCHAR NOT NULL,
  api_id VARCHAR NOT NULL
);
CREATE TABLE users (
  id INTEGER PRIMARY KEY NOT NULL,
  tg_id INTEGER NOT NULL,
  tg_name VARCHAR NOT NULL,
  notify BOOLEAN NOT NULL DEFAULT 0,
  group_id INTEGER NOT NULL,
  FOREIGN KEY(group_id) REFERENCES groups(group_id)
);
