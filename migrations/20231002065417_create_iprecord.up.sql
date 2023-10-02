create table if not exists strikes (
    origin TINYBLOB not null,
    amount SMALLINT unsigned not null
);

create table if not exists origins (
    id TEXT not null,
    created_by TINYBLOB not null
);
