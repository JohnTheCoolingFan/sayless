create table if not exists links (
    id TEXT not null,
    hash BLOB not null,
    link TEXT not null,
    created_at TIMESTAMP not null default CURRENT_TIMESTAMP
);

create table if not exists origins (
    id TEXT not null,
    created_by TINYBLOB not null
);

create table if not exists strikes (
    origin TINYBLOB not null,
    amount SMALLINT unsigned not null
);

create table if not exists tokens (
    token TEXT not null,
    created_at TIMESTAMP default CURRENT_TIMESTAMP not null,
    expires_at TIMESTAMP default (CURRENT_TIMESTAMP + INTERVAL 1 YEAR) not null,
    admin_perm BOOLEAN not null,
    create_link_perm BOOLEAN not null,
    create_token_perm BOOLEAN not null,
    view_ips_perm BOOLEAN not null
);
