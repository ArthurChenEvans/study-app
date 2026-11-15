-- Shudhana sync through Supabase (R206 to R210).
--
-- Each computer appends its change lines (the same ones folder sync writes) to `changes`, and
-- says how far it has got in `devices`. Every row belongs to one account, and Row Level
-- Security means an account only ever sees and writes its own rows, whatever the app sends.
--
-- Run this once in the Supabase dashboard: SQL Editor, New query, paste, Run.

-- Any table made from now on gets Row Level Security switched on, even one made by hand, so a
-- table can never be left open by mistake. (A table with RLS on and no policy is closed.)
create or replace function public.rls_auto_enable()
returns event_trigger
language plpgsql
security definer
set search_path = ''
as $$
declare
  made record;
begin
  for made in select * from pg_event_trigger_ddl_commands() where command_tag = 'CREATE TABLE' and schema_name = 'public' loop
    execute format('alter table %s enable row level security', made.object_identity);
  end loop;
end;
$$;

drop event trigger if exists rls_auto_enable;
create event trigger rls_auto_enable on ddl_command_end when tag in ('CREATE TABLE') execute function public.rls_auto_enable();

-- The computers signed in to an account, and how far each one's lines go.
create table public.devices (
  user_id uuid not null default auth.uid() references auth.users (id) on delete cascade,
  device text not null check (device ~ '^[A-Za-z0-9_-]{1,64}$'),
  name text not null check (char_length(name) between 1 and 80),
  seq bigint not null default 0 check (seq >= 0),
  seen_at timestamptz not null default now(),
  app text not null default '' check (char_length(app) <= 40),
  primary key (user_id, device)
);

-- Each computer's change lines, only ever added to. A line is capped in size so one account
-- can't fill the database with a single row.
create table public.changes (
  user_id uuid not null default auth.uid() references auth.users (id) on delete cascade,
  device text not null check (device ~ '^[A-Za-z0-9_-]{1,64}$'),
  seq bigint not null check (seq > 0),
  line jsonb not null check (pg_column_size(line) < 262144),
  added_at timestamptz not null default now(),
  primary key (user_id, device, seq)
);

-- Settings for everyone, changed only by an admin. One row.
create table public.app_settings (
  id smallint primary key default 1 check (id = 1),
  images_allowed boolean not null default false
);
insert into public.app_settings (id) values (1) on conflict do nothing;

-- Who's an admin. Added by hand in the Table Editor, never from the app.
create table public.admins (
  user_id uuid primary key references auth.users (id) on delete cascade
);

alter table public.devices enable row level security;
alter table public.changes enable row level security;
alter table public.app_settings enable row level security;
alter table public.admins enable row level security;

-- Your own rows, and nobody else's.
create policy "own devices" on public.devices for all to authenticated
  using (user_id = (select auth.uid())) with check (user_id = (select auth.uid()));
create policy "read own changes" on public.changes for select to authenticated
  using (user_id = (select auth.uid()));
create policy "add own changes" on public.changes for insert to authenticated
  with check (user_id = (select auth.uid()));
-- No update policy on changes: a line, once written, never changes.

create policy "everyone reads settings" on public.app_settings for select to authenticated using (true);
create policy "admins change settings" on public.app_settings for update to authenticated
  using (exists (select 1 from public.admins where user_id = (select auth.uid())))
  with check (exists (select 1 from public.admins where user_id = (select auth.uid())));

create policy "see if you're an admin" on public.admins for select to authenticated
  using (user_id = (select auth.uid()));

-- Two-step sign-in: once someone has turned it on, their data needs the second step too.
-- Signed-in users can't read auth.mfa_factors, so the check is a function that can.
create or replace function public.two_step_ok()
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select coalesce(auth.jwt() ->> 'aal', '') = 'aal2'
      or not exists (select 1 from auth.mfa_factors where user_id = auth.uid() and status = 'verified');
$$;

revoke all on function public.two_step_ok() from public, anon;
grant execute on function public.two_step_ok() to authenticated;

create policy "two-step when it's on" on public.devices as restrictive for all to authenticated
  using ((select public.two_step_ok())) with check ((select public.two_step_ok()));
create policy "two-step when it's on" on public.changes as restrictive for all to authenticated
  using ((select public.two_step_ok())) with check ((select public.two_step_ok()));

-- Deleting your own account (GDPR): the account, and through it every row above. Asks for the
-- second step when two-step sign-in is on.
create or replace function public.delete_my_account()
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  me uuid := auth.uid();
  needs_two_step boolean;
begin
  if me is null then
    raise exception 'not signed in';
  end if;
  select exists (select 1 from auth.mfa_factors where user_id = me and status = 'verified') into needs_two_step;
  if needs_two_step and coalesce(auth.jwt() ->> 'aal', '') <> 'aal2' then
    raise exception 'two-step sign-in needed';
  end if;
  delete from auth.users where id = me;
end;
$$;

revoke all on function public.delete_my_account() from public, anon;
grant execute on function public.delete_my_account() to authenticated;
revoke all on function public.rls_auto_enable() from public, anon, authenticated;
