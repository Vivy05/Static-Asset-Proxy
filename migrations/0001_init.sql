create table if not exists tenants (
    id uuid primary key,
    name text not null,
    slug text not null unique
);

create table if not exists users (
    id uuid primary key,
    tenant_id uuid not null references tenants(id),
    email text not null unique,
    display_name text not null
);

create table if not exists sites (
    id uuid primary key,
    tenant_id uuid not null references tenants(id),
    owner_user_id uuid not null references users(id),
    name text not null,
    default_entry_path text not null,
    file_count bigint not null default 0,
    total_size_bytes bigint not null default 0,
    storage_key text null,
    deployed_by_user_id uuid null references users(id)
);

create index if not exists idx_sites_tenant_id on sites(tenant_id);
create index if not exists idx_sites_owner_user_id on sites(owner_user_id);
create index if not exists idx_sites_deployed_by_user_id on sites(deployed_by_user_id);
