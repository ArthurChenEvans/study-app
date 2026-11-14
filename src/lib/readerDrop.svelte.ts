// R195: files dropped on the reader in the desktop app. The window's drop handler reads them by
// path and leaves them here, and the reader opens them.
import { api } from "./api";

export const readerDrop = $state<{ files: File[] }>({ files: [] });

export async function dropOnReader(paths: string[]): Promise<void> {
  const files: File[] = [];
  for (const path of paths) {
    const picked = await api.readFileAt(path);
    const bytes = Uint8Array.from(atob(picked.data), (letter) => letter.charCodeAt(0));
    files.push(new File([bytes], picked.name));
  }
  readerDrop.files = [...readerDrop.files, ...files];
}
