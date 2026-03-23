// ── Shared UI primitives ──────────────────────────────────────────────────────
export function SectionHeader({ title }: { title: string }) {
  return (
    <h2 class="text-xs font-semibold uppercase tracking-wider text-gray-400">
      {title}
    </h2>
  );
}
