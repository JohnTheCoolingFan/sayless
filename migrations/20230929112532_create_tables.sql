create table if not exists links (
    id TEXT not null,
    hash BLOB not null,
    link TEXT not null,
    created_at TIMESTAMP not null default CURRENT_TIMESTAMP
);
