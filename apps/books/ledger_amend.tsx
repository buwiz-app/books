// Amend by reversal (ledger-3): the entry drawer's Amend button and its
// link chips. An amend is a void with `amend: true` (no new tool): the
// original shows Voided, a reversal posts, and a replacement draft with the
// same lines opens for the user to correct and post. Only a click here
// amends (DD-20). The reason goes to Books and is never shown back.

/** "Amend" in the drawer of a posted entry: a reason and an optional date. */
function AmendButton(p: { entry: string; busy: boolean }) {
  const open = signal(false);
  const reason = signal("");
  const date = signal("");
  const saving = signal(false);
  const close = () => {
    open.set(false);
    reason.set("");
    date.set("");
  };
  const amend = () => {
    const a: Record<string, Any> = { entry: p.entry, amend: true, reason: reason().trim() };
    if (date().trim()) a.date = date().trim();
    saving.set(true);
    act("books_app_void_entry", a)
      .then((r: Any) => {
        close();
        drawerEntry.set(r.replacement);
        toast(`Draft ${r.replacement_no} saved; the original is voided`);
      })
      .catch(() => null)
      .finally(() => saving.set(false));
  };
  return (
    <View>
      <Btn label="Amend" disabled={p.busy || saving()} onPress={() => open.set(true)} />
      <If when={open()}>
        <Overlay anchor={{ x: 24, y: 96 }} placement="bottom" align="start" offset={4} margin={8} onCancel={close} onPressOutside={close}>
          <View class={`w-[300px] gap-2.5 p-3.5 rounded-xl border ${C.panel} ${C.line}`}>
            <Text class={`text-[13px] font-bold ${C.ink}`}>Amend this entry</Text>
            <Text class={`text-[12px] ${C.dim}`}>It is voided with a reversal, and a draft with the same lines opens for you to correct.</Text>
            <Field label="Reason">
              <TextBox value={reason()} onChange={(v: string) => reason.set(v)} placeholder="Why it changes (3 to 200 characters)" />
            </Field>
            <Field label="Date (optional)">
              <TextBox value={date()} onChange={(v: string) => date.set(v)} placeholder="YYYY-MM-DD" />
            </Field>
            <View class="flex-row gap-2">
              <Btn label="Cancel" onPress={close} />
              <Btn label="Amend" tone="go" disabled={saving() || reason().trim().length < 3} onPress={amend} />
            </View>
          </View>
        </Overlay>
      </If>
    </View>
  );
}

/** "Amends entry N" / "Amended by entry N" chips; a press opens that entry. */
function AmendLinks(p: { e: Any }) {
  const chip = (key: string) => (
    <If when={Boolean(p.e?.links?.[key])}>
      <Btn label={p.e?.link_labels?.[key] || ""} small onPress={() => drawerEntry.set(p.e?.links?.[key])} />
    </If>
  );
  return (
    <View class="flex-row gap-2 flex-wrap">
      {chip("amends")}
      {chip("amended_by")}
    </View>
  );
}
