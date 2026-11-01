// Reconcile flags and reopen (M10 parity): display-only notes about a
// statement's lines, and the reasoned reopen of a finished month.

/** An amber panel listing the statement's flags; they never block Finish. */
function FlagsNote(p: { flags: Any[] }) {
  return (
    <Panel pad class="gap-1 border-[#a2650f]">
      <Text class={`text-[12.5px] font-semibold ${C.amber}`}>Worth a look before you finish</Text>
      <Each of={p.flags || []}>
        {(f: Any) => (
          <Text class={`text-[12px] ${C.amber}`}>{f().line ? `Line ${String(f().line)}: ${f().note}` : f().note}</Text>
        )}
      </Each>
    </Panel>
  );
}

/** Reopen a finished month's statement, with the user's reason. */
function ReopenFooter(p: { onReopen: (reason: string) => void }) {
  const reason = signal("");
  return (
    <Panel pad class="flex-row gap-2 items-end">
      <Field label="Why reopen this month's statement?" grow>
        <TextBox value={reason()} onChange={(v) => reason.set(v)} placeholder="A line was matched to the wrong entry" />
      </Field>
      <Btn label="Reopen this month's statement" disabled={reason().trim().length < 3} onPress={() => p.onReopen(reason())} />
    </Panel>
  );
}
