import { apiFetch } from './core';

export interface KvEntry {
  key: string;
  value: string;
}

export async function listKvEntries(): Promise<KvEntry[]> {
  return apiFetch('/kv');
}

export async function upsertKvEntry(key: string, value: string): Promise<KvEntry> {
  return apiFetch('/kv', {
    method: 'POST',
    body: JSON.stringify({ key, value }),
  });
}

export async function deleteKvEntry(key: string): Promise<{ key: string; deleted: boolean }> {
  return apiFetch('/kv', {
    method: 'DELETE',
    body: JSON.stringify({ key }),
  });
}
