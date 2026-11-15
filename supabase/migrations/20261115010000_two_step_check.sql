-- For a project set up with the first version of 20261115000000_sync.sql: its two-step
-- policies read auth.mfa_factors directly, which signed-in users aren't allowed to do
-- ("permission denied for table mfa_factors"). This moves the check into a function that can.
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

drop policy if exists "two-step when it's on" on public.devices;
drop policy if exists "two-step when it's on" on public.changes;

create policy "two-step when it's on" on public.devices as restrictive for all to authenticated
  using ((select public.two_step_ok())) with check ((select public.two_step_ok()));
create policy "two-step when it's on" on public.changes as restrictive for all to authenticated
  using ((select public.two_step_ok())) with check ((select public.two_step_ok()));
