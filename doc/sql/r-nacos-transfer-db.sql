
create table if not exists tb_config(
    id integer primary key autoincrement,
    data_id text,
    group_id text,
    tenant_id text,
    content text,
    config_type text,
    config_desc text,
    last_time long
);
create index if not exists tb_config_key_idx on tb_config(data_id,group_id,tenant_id);

create table if not exists tb_config_history(
    id integer primary key autoincrement,
    data_id text,
    group_id text,
    tenant_id text,
    content text,
    config_type text,
    config_desc text,
    op_user text,
    last_time long
);
create index if not exists tb_config_history_key_idx on tb_config_history(data_id,group_id,tenant_id);

create table if not exists tb_tenant(
    id integer primary key autoincrement,
    tenant_id text,
    tenant_name text,
    tenant_desc text,
    create_flag integer
);

create table if not exists tb_user(
    id integer primary key autoincrement,
    username text,
    nickname text,
    password_hash text,
    gmt_create integer,
    gmt_modified integer,
    enabled text,
    roles text,
    extend_info text
);

