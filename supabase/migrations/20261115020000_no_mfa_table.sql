-- For a project set up with an earlier 20261115000000_sync.sql. Newer Supabase projects don't
-- let even the admin role read auth.mfa_factors, so the two-step policies refused every request
-- ("permission denied for table mfa_factors"). The database stops checking two-step sign-in
-- (the app still asks for it), and deleting your account no longer reads that table.
drop policy if exists "two-step when it's on" on public.devices;
drop policy if exists "two-step when it's on" on public.changes;
drop function if exists public.two_step_ok();

create or replace function public.delete_my_account()
returns void
language plpgsql
security definer
set search_path = ''
as $$
begin
  if auth.uid() is null then
    raise exception 'not signed in';
  end if;
  delete from auth.users where id = auth.uid();
end;
$$;

revoke all on function public.delete_my_account() from public, anon;
grant execute on function public.delete_my_account() to authenticated;
