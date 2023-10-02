create table if not exists tokens (
    token TEXT not null,
    created_at TIMESTAMP default CURRENT_TIMESTAMP not null,
    expires_at TIMESTAMP default (CURRENT_TIMESTAMP + INTERVAL 1 YEAR) not null,
    admin_perm BOOLEAN not null,
    create_link_perm BOOLEAN not null,
    create_token_perm BOOLEAN not null,
    view_ips_perm BOOLEAN not null
);
