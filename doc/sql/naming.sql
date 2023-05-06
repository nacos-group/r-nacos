

create table if not exists tb_service(
    id integer primary key autoincrement,
    namespace_id varchar(255),
    service_name varchar(255),
    group_name varchar(255),
    instance_size integer,
    healthy_size integer,
    threshold float,
    metadata text,
    extend_info text,
    create_time long,
    last_time long
);
create index if not exists tb_service_key_idx on tb_service(namespace_id,service_name,group_name);
