// Certificates: the 2307s customers sent, with status. Only a received 2307
// counts toward a form; a 2306 is final tax and never does. Building SAWT is
// the BIR app's (drawing: Books Mini App › Certificates).

const certFilter = signal("all");
const certSheet = signal(false);

const CERT_TONE: Record<string, string> = { received: "ok", pending: "draft", lost: "rev", disputed: "rev" };

function CertificatesPage(p: { go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver(), f: certFilter() }),
    (k: Any) => rox.call("books_certificates_list", { filter: k.f }),
  );
  const d = () => data() as Any;
  const changing = signal("");
  return (
    <View class="gap-3.5">
      <Head title="Certificates" sub="2307s your customers sent you">
        <Btn label="+ Add a 2307" tone="go" onPress={() => certSheet.set(true)} />
      </Head>
      <View class="flex-row gap-1.5">
        <Chip label="All" count={d()?.counts.all} on={certFilter() === "all"} onPress={() => certFilter.set("all")} />
        <Chip label="Received" count={d()?.counts.received} on={certFilter() === "received"} onPress={() => certFilter.set("received")} />
        <Chip label="Pending" count={d()?.counts.pending} on={certFilter() === "pending"} onPress={() => certFilter.set("pending")} />
      </View>
      <Loading res={data} />
      <If when={Boolean(d())}>
        <If when={certSheet()}>
          <CertSheet opts={d()} close={() => certSheet.set(false)} />
        </If>
        <Panel>
          <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>From</Text>
            <Text class={`w-[70px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Quarter</Text>
            <Text class={`w-[120px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Kind</Text>
            <Text class={`w-[100px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Withheld</Text>
            <Text class={`w-[90px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Status</Text>
            <Text class={`w-[180px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Counts toward</Text>
          </View>
          <If when={(d()?.rows || []).length === 0}>
            <Text class={`text-[12.5px] px-3 py-4 ${C.dim}`}>No certificates here yet.</Text>
          </If>
          <Each of={d()?.rows || []} key={rowKey}>
            {(c: Any) => (
              <View class={`border-b ${C.line}`}>
                <View class="flex-row items-center gap-3 px-3 py-2.5">
                  <View class="flex-1">
                    <Text class={`text-[13px] font-semibold ${C.ink}`} numberOfLines={1}>
                      {c().from}
                    </Text>
                    <Text class={`text-[11.5px] ${C.dim}`}>{`ATC on the paper: ${String(c().atc || "—")}`}</Text>
                  </View>
                  <Text class={`w-[70px] text-[12.5px] font-mono ${C.ink}`}>{c().quarter}</Text>
                  <Text class={`w-[120px] text-[12.5px] ${C.ink}`}>{c().kind}</Text>
                  <Text class={`w-[100px] text-right text-[12.5px] font-mono ${C.ink}`}>{c().amount_shown}</Text>
                  <View class="w-[90px]" onPress={() => changing.set(changing() === c().ticket ? "" : c().ticket)}>
                    <Pill tone={CERT_TONE[c().status] || "mute"} label={c().status} />
                  </View>
                  <View class="w-[180px]">
                    <If when={c().counts_bir}>
                      <Pill tone="bir" label={c().counts} />
                    </If>
                    <If when={!c().counts_bir}>
                      <Text class={`text-[12px] ${C.dim}`}>{c().counts}</Text>
                    </If>
                  </View>
                </View>
                <If when={changing() === c().ticket}>
                  <View class="flex-row items-center gap-2 px-3 pb-2.5">
                    <Text class={`text-[12px] ${C.dim}`}>The paper is</Text>
                    <Each of={["received", "pending", "lost", "disputed"]}>
                      {(s: Any) => (
                        <Btn
                          label={s()}
                          small
                          tone={s() === c().status ? "go" : "plain"}
                          onPress={() =>
                            act("books_app_certificate_set", { certificate: c().ticket, status: s() })
                              .then(() => changing.set(""))
                              .catch(() => null)
                          }
                        />
                      )}
                    </Each>
                  </View>
                </If>
              </View>
            )}
          </Each>
        </Panel>
        <Panel pad class="flex-row items-center gap-2.5">
          <Pill tone="bir" label="BIR" />
          <Text class={`flex-1 text-[12.5px] ${C.mute}`}>Only received 2307s count. A 2306 is final tax and never counts. The BIR app builds SAWT and keeps the TINs.</Text>
        </Panel>
      </If>
    </View>
  );
}

function CertSheet(p: { opts: Any; close: () => void }) {
  const person = signal("");
  const form = signal("2307");
  const kind = signal("pt");
  const atc = signal("");
  const quarter = signal("");
  const amount = signal("");
  const status = signal("received");
  const recorded = signal("net");
  const against = signal("");
  const err = signal("");
  const save = () => {
    err.set("");
    const a = moneyText(amount());
    if (!person()) return err.set("Pick the customer who sent it.");
    if (!a) return err.set("Type the amount withheld, like 3,000.00.");
    const args: Record<string, Any> = { person: person(), form: form(), credit_kind: kind(), atc: atc(), quarter: quarter().trim(), amount: a, status: status() };
    if (recorded() === "full" && !against()) return err.set("Pick the account the sale's money went into.");
    if (against()) args.against = against();
    act("books_app_certificate_add", args)
      .then(() => {
        toast("Certificate posted");
        p.close();
      })
      .catch((e: Any) => err.set(errText(e)));
  };
  return (
    <Panel pad class="gap-3">
      <View class="flex-row items-center justify-between">
        <Text class={`text-[14px] font-bold ${C.ink}`}>Add a certificate</Text>
        <Btn label="Close" small onPress={p.close} />
      </View>
      <Text class={`text-[12px] ${C.mute}`}>Copy what the paper prints. A sale recorded net of the tax gets the withheld part added to sales; one recorded in full gives it back from the bank.</Text>
      <View class="flex-row gap-2 items-end flex-wrap">
        <View class="w-[220px]">
          <Field label="From">
            <Pick value={person()} options={p.opts?.customers || []} onPick={(o: Any) => person.set(o.ticket)} placeholder="Customer" />
          </Field>
        </View>
        <Field label="Form">
          <Seg
            options={[
              ["2307", "2307"],
              ["2306", "2306 (final)"],
            ]}
            value={form()}
            onPick={form.set}
          />
        </Field>
        <Field label="Kind">
          <Seg
            options={[
              ["pt", "Percentage tax"],
              ["it", "Income tax"],
            ]}
            value={kind()}
            onPick={kind.set}
          />
        </Field>
      </View>
      <View class="flex-row gap-2 items-end flex-wrap">
        <View class="w-[130px]">
          <Field label="ATC on the paper">
            <Pick value={atc()} options={p.opts?.atc || []} onPick={(o: Any) => atc.set(o.ticket)} placeholder="As printed" />
          </Field>
        </View>
        <View class="w-[110px]">
          <Field label="Quarter">
            <TextBox value={quarter()} onChange={quarter.set} placeholder="2026-Q3" />
          </Field>
        </View>
        <View class="w-[130px]">
          <Field label="Withheld">
            <TextBox value={amount()} onChange={amount.set} placeholder="0.00" />
          </Field>
        </View>
        <Field label="Status">
          <Seg
            options={[
              ["received", "Received"],
              ["pending", "Pending"],
            ]}
            value={status()}
            onPick={status.set}
          />
        </Field>
        <Field label="The sale was recorded">
          <Seg
            options={[
              ["net", "Net of the tax"],
              ["full", "In full, as money in"],
            ]}
            value={recorded()}
            onPick={(x: string) => {
              recorded.set(x);
              against.set("");
            }}
          />
        </Field>
        <View class="w-[220px]">
          <Field label={recorded() === "net" ? "Adds to" : "Comes out of"}>
            <Pick
              value={against()}
              options={recorded() === "net" ? p.opts?.revenue_accounts || [] : p.opts?.money_accounts || []}
              onPick={(o: Any) => against.set(o.ticket)}
              placeholder={recorded() === "net" ? "Sales Revenue" : "Bank or cash"}
            />
          </Field>
        </View>
      </View>
      <View class="flex-row justify-end">
        <Btn label="Post it" tone="go" onPress={save} />
      </View>
      <If when={Boolean(err())}>
        <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
      </If>
    </Panel>
  );
}
