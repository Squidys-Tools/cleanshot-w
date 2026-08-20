export function normalizeCaptureTitle(title: string): string {
  return title.trim();
}

export function filterCaptureRecords<T extends { title: string }>(records: T[], query: string): T[] {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) return records;
  return records.filter((record) => record.title.toLocaleLowerCase().includes(normalized));
}
