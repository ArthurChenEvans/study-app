-- R221: signing in on a computer that has its own data, to an account that already has some,
-- asks which to keep. Keeping the computer's clears what the account had synced, so the
-- computer's data goes up on its own. Changes have no delete policy (a line, once written,
-- never changes), so this runs as the owner and only ever touches the caller's own rows.
create or replace function public.clear_my_sync()
returns void
language plpgsql
security definer
set search_path = ''
as $$
begin
  if auth.uid() is null then
    raise exception 'not signed in';
  end if;
  delete from public.changes where user_id = auth.uid();
  delete from public.devices where user_id = auth.uid();
end;
$$;

revoke all on function public.clear_my_sync() from public, anon;
grant execute on function public.clear_my_sync() to authenticated;
