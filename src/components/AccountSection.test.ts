import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async (original) => {
  const actual = await original<typeof import("../lib/api")>();
  return {
    ...actual,
    api: {
      cloudStatus: vi.fn(), cloudRequestCode: vi.fn(), cloudVerifyCode: vi.fn(), cloudVerifyTwoStep: vi.fn(), cloudTwoStepStart: vi.fn(), cloudTwoStepConfirm: vi.fn(),
      cloudTwoStepOff: vi.fn(), cloudSignOut: vi.fn(), cloudSignOutOthers: vi.fn(), cloudDeleteAccount: vi.fn(), cloudSetImages: vi.fn(), syncNow: vi.fn(),
    },
  };
});

import { api, type CloudStatus } from "../lib/api";
import AccountSection from "./AccountSection.svelte";

const mocked = vi.mocked(api);
const out: CloudStatus = { signed_in: false, email: null, two_step: false, needs_two_step: false, images_allowed: true, admin: false, devices: [], last_synced: null, problem: null };
const signedIn: CloudStatus = { ...out, signed_in: true, email: "chen@example.com", images_allowed: false, devices: [{ id: "a", name: "Desk", this_one: true, last_seen: 1_800_000_000 }, { id: "b", name: "Laptop", this_one: false, last_seen: 1_799_990_000 }], last_synced: 1_800_000_000 };

beforeEach(() => {
  vi.resetAllMocks();
});

// R206: sign in with a code emailed to you, no password
describe("Account", () => {
  it("signs in with an emailed code", async () => {
    const u = userEvent.setup();
    mocked.cloudStatus.mockResolvedValue(out);
    mocked.cloudRequestCode.mockResolvedValue(out);
    mocked.cloudVerifyCode.mockResolvedValue(signedIn);
    render(AccountSection);
    await u.click(await screen.findByRole("button", { name: "Sign in to sync" }));
    await u.type(screen.getByLabelText("Email"), "chen@example.com");
    await u.click(screen.getByRole("button", { name: "Email me a code" }));
    expect(mocked.cloudRequestCode).toHaveBeenCalledWith("chen@example.com");
    expect(await screen.findByText(/We sent a 6-digit code to/)).toHaveTextContent("chen@example.com");
    const code = screen.getByLabelText("Code from the email");
    expect(code).toHaveAttribute("autocomplete", "one-time-code");
    expect(code).toHaveAttribute("inputmode", "numeric");
    expect(code.closest(".otp")).not.toBeNull();
    await u.type(code, "482913");
    await u.click(screen.getByRole("button", { name: "Sign in" }));
    expect(mocked.cloudVerifyCode).toHaveBeenCalledWith("chen@example.com", "482913");
    expect(await screen.findByText("chen@example.com")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sync now" })).toBeInTheDocument();
  });

  it("asks for the authenticator code when two-step sign-in is on, and shows errors", async () => {
    const u = userEvent.setup();
    mocked.cloudStatus.mockResolvedValue(out);
    mocked.cloudRequestCode.mockResolvedValue(out);
    mocked.cloudVerifyCode.mockResolvedValue({ ...signedIn, two_step: true, needs_two_step: true, devices: [] });
    mocked.cloudVerifyTwoStep.mockRejectedValueOnce({ message: "that code didn't work. Codes change every 30 seconds, so use the newest one" }).mockResolvedValueOnce({ ...signedIn, two_step: true });
    render(AccountSection);
    await u.click(await screen.findByRole("button", { name: "Sign in to sync" }));
    await u.type(screen.getByLabelText("Email"), "chen@example.com{Enter}");
    await u.type(await screen.findByLabelText("Code from the email"), "482913{Enter}");
    const box = await screen.findByLabelText("Code from your authenticator app");
    await u.type(box, "111111{Enter}");
    expect(await screen.findByRole("alert")).toHaveTextContent("Codes change every 30 seconds");
    await u.clear(box);
    await u.type(box, "222222{Enter}");
    expect(mocked.cloudVerifyTwoStep).toHaveBeenLastCalledWith("222222");
    expect(await screen.findByRole("button", { name: "Sync now" })).toBeInTheDocument();
  });

  // R208: two-step sign-in with an authenticator app, opt-in
  it("turns two-step sign-in on with a QR code", async () => {
    const u = userEvent.setup();
    mocked.cloudStatus.mockResolvedValue(signedIn);
    mocked.cloudTwoStepStart.mockResolvedValue({ factor: "f1", qr: "data:image/svg+xml;utf-8,<svg/>", secret: "ABCDEF" });
    mocked.cloudTwoStepConfirm.mockResolvedValue({ ...signedIn, two_step: true });
    render(AccountSection);
    await u.click(await screen.findByRole("button", { name: "Turn on two-step sign-in" }));
    const dialog = await screen.findByRole("dialog", { name: "Turn on two-step sign-in" });
    expect(within(dialog).getByRole("img", { name: /QR code/ })).toHaveAttribute("src", "data:image/svg+xml;utf-8,<svg/>");
    expect(within(dialog).getByText("ABCDEF")).toBeInTheDocument();
    await u.type(within(dialog).getByLabelText("Code from your authenticator app"), "123456");
    await u.click(within(dialog).getByRole("button", { name: "Turn on" }));
    expect(mocked.cloudTwoStepConfirm).toHaveBeenCalledWith("f1", "123456");
    expect(await screen.findByText(/Two-step sign-in is on/)).toBeInTheDocument();
  });

  it("signs out, keeping this computer's data, and signs other computers out", async () => {
    const u = userEvent.setup();
    mocked.cloudStatus.mockResolvedValue(signedIn);
    mocked.cloudSignOutOthers.mockResolvedValue({ ...signedIn, devices: [signedIn.devices[0]] });
    mocked.cloudSignOut.mockResolvedValue(out);
    render(AccountSection);
    const devices = await screen.findByRole("list", { name: "Signed in on" });
    expect(within(devices).getAllByRole("listitem").map((item) => item.textContent)).toEqual([expect.stringContaining("Desk"), expect.stringContaining("Laptop")]);
    expect(within(devices).getByText("This computer")).toBeInTheDocument();
    await u.click(screen.getByRole("button", { name: "Sign out everywhere else" }));
    expect(mocked.cloudSignOutOthers).toHaveBeenCalled();
    await u.click(screen.getByRole("button", { name: "Sign out…" }));
    const dialog = screen.getByRole("dialog", { name: "Sign out of chen@example.com?" });
    expect(dialog).toHaveTextContent("Everything stays on this computer");
    await u.click(within(dialog).getByRole("button", { name: "Sign out" }));
    expect(mocked.cloudSignOut).toHaveBeenCalled();
    expect(await screen.findByRole("button", { name: "Sign in to sync" })).toBeInTheDocument();
  });

  // R209: delete the account and everything synced
  it("deletes the account only after typing DELETE", async () => {
    const u = userEvent.setup();
    mocked.cloudStatus.mockResolvedValue(signedIn);
    mocked.cloudDeleteAccount.mockResolvedValue(out);
    render(AccountSection);
    await u.click(await screen.findByRole("button", { name: "Delete account…" }));
    const dialog = screen.getByRole("dialog", { name: "Delete your account?" });
    const confirm = within(dialog).getByRole("button", { name: "Delete account" });
    expect(confirm).toBeDisabled();
    await u.type(within(dialog).getByLabelText("Type DELETE to confirm"), "DELETE");
    await u.click(confirm);
    expect(mocked.cloudDeleteAccount).toHaveBeenCalled();
  });

  // R210: only the admin sees the image switch
  it("shows the admin the image switch, and nobody else", async () => {
    const u = userEvent.setup();
    mocked.cloudStatus.mockResolvedValue(signedIn);
    const { unmount } = render(AccountSection);
    await screen.findByText("chen@example.com");
    expect(screen.queryByRole("checkbox", { name: /Let people add images/ })).not.toBeInTheDocument();
    unmount();
    mocked.cloudStatus.mockResolvedValue({ ...signedIn, admin: true });
    mocked.cloudSetImages.mockResolvedValue({ ...signedIn, admin: true, images_allowed: true });
    render(AccountSection);
    const box = await screen.findByRole("checkbox", { name: /Let people add images/ });
    expect(box).not.toBeChecked();
    await u.click(box);
    expect(mocked.cloudSetImages).toHaveBeenCalledWith(true);
  });
});
