// Payroll: pay runs and their gross pay, amounts only. Employee TINs, 2316s,
// the tax table and the 1601-C filing are in the BIR app. A new run saves
// as a draft with each employee's pay as payroll recorded it; Post makes it
// count (drawing: Books Mini App › Payroll).

const runSheet = signal(false);

const PAY_FIELDS: [string, string][] = [
  ["basic", "Basic pay"],
  ["minimum_wage", "Minimum wage pay"],
  ["overtime", "Overtime, holiday"],
  ["thirteenth", "13th month"],
  ["de_minimis", "De minimis"],
  ["ee_sss", "Employee SSS"],
  ["ee_philhealth", "Employee PhilHealth"],
  ["ee_pagibig", "Employee Pag-IBIG"],
  ["er_sss", "Employer SSS"],
  ["er_philhealth", "Employer PhilHealth"],
  ["er_pagibig", "Employer Pag-IBIG"],
  ["tax_withheld", "Tax withheld"],
];

function PayrollPage(p: { go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver() }),
    () => rox.call("books_wages_list", {}),
  );
  const d = () => data() as Any;
  return (
    <View class="gap-3.5">
      <Head title="Payroll" sub="pay runs">
        <Btn label="+ New pay run" tone="go" onPress={() => runSheet.set(true)} />
      </Head>
      <Loading res={data} />
      <If when={Boolean(d())}>
        <Panel>
          <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Run</Text>
            <Text class={`w-[100px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>People</Text>
            <Text class={`w-[110px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Gross pay</Text>
            <Text class={`w-[90px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Status</Text>
            <Text class={`w-[170px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>For the BIR app</Text>
          </View>
          <If when={(d()?.runs || []).length === 0}>
            <Text class={`text-[12.5px] px-3 py-4 ${C.dim}`}>No pay runs yet.</Text>
          </If>
          <Each of={d()?.runs || []} key={rowKey}>
            {(r: Any) => (
              <View class={`flex-row items-center gap-3 px-3 py-2.5 border-b ${C.line}`}>
                <View class="flex-1">
                  <Text class={`text-[13px] font-semibold ${C.ink}`}>{r().label}</Text>
                  <Text class={`text-[11.5px] ${C.dim}`}>{`paid ${String(r().date)}`}</Text>
                </View>
                <Text class={`w-[100px] text-[12.5px] ${C.ink}`}>{r().people}</Text>
                <Text class={`w-[110px] text-right text-[12.5px] font-mono ${C.ink}`}>{r().gross_shown}</Text>
                <View class="w-[90px] gap-1">
                  <Pill tone={r().tone} label={r().status_label} />
                  <If when={r().status === "draft"}>
                    <Btn label="Post" tone="go" small onPress={() => act("books_app_post_entry", { entry: r().ticket }).then(() => toast("Pay run posted")).catch(() => null)} />
                  </If>
                </View>
                <View class="w-[170px]">
                  <Pill tone={r().bir_tone} label={r().bir} />
                </View>
              </View>
            )}
          </Each>
        </Panel>
        <Panel pad class="flex-row items-center gap-2.5">
          <Pill tone="bir" label="BIR" />
          <Text class={`flex-1 text-[12.5px] ${C.mute}`}>Books records each line of pay. Employee TINs, 2316s, the tax table and the 1601-C filing are in the BIR app.</Text>
        </Panel>
        <If when={runSheet()}>
          <PayRunSheet employees={d()?.employees || []} money={d()?.money_accounts || []} close={() => runSheet.set(false)} go={p.go} />
        </If>
      </If>
    </View>
  );
}

function PayRunSheet(p: { employees: Any[]; money: Any[]; close: () => void; go: (page: string) => void }) {
  const from = signal("");
  const to = signal("");
  const account = signal("");
  const lines = signal([{ person: "" } as Record<string, string>]);
  const err = signal("");
  const set = (i: number, k: string, val: string) => {
    const next = lines().map((x: Any, j: number) => (j === i ? { ...x, [k]: val } : x));
    lines.set(next);
  };
  const save = () => {
    err.set("");
    const employees: Any[] = [];
    for (const l of lines()) {
      if (!l.person) return err.set("Pick each employee.");
      const e: Record<string, string> = { person: l.person };
      for (const [k] of PAY_FIELDS) {
        if (!l[k]) continue;
        const m = moneyText(l[k]);
        if (!m) return err.set("Type amounts like 40,000.00.");
        e[k] = m;
      }
      employees.push(e);
    }
    act("books_app_wage_run_save", { period_from: from().trim(), period_to: to().trim(), account: account(), employees })
      .then((r: Any) => {
        toast(`Saved as a draft · net pay ${String(r.net_shown)}`);
        p.close();
      })
      .catch((e: Any) => err.set(errText(e)));
  };
  return (
    <Panel pad class="gap-3">
      <View class="flex-row items-center justify-between">
        <Text class={`text-[14px] font-bold ${C.ink}`}>New pay run</Text>
        <Btn label="Close" small onPress={p.close} />
      </View>
      <If when={p.employees.length === 0}>
        <View class="flex-row items-center gap-2">
          <Text class={`text-[12.5px] ${C.amber}`}>Add your employees on the People page first.</Text>
          <Btn label="People" small onPress={() => p.go("people")} />
        </View>
      </If>
      <View class="flex-row gap-2 items-end">
        <View class="w-[120px]">
          <Field label="Period from">
            <TextBox value={from()} onChange={from.set} placeholder="2026-09-01" />
          </Field>
        </View>
        <View class="w-[120px]">
          <Field label="to">
            <TextBox value={to()} onChange={to.set} placeholder="2026-09-30" />
          </Field>
        </View>
        <Field label="Net pay leaves" grow>
          <Pick value={account()} options={p.money} onPick={(a: Any) => account.set(a.ticket)} placeholder="Bank or cash" />
        </Field>
      </View>
      <Each of={lines()}>
        {(l: Any, i: Any) => (
          <View class={`rounded-[10px] border p-2.5 gap-2 ${C.line}`}>
            <View class="w-[260px]">
              <Pick value={l().person || ""} options={p.employees} onPick={(o: Any) => set(i(), "person", o.ticket)} placeholder="Employee" />
            </View>
            <View class="flex-row flex-wrap gap-2">
              <Each of={PAY_FIELDS}>
                {(f: Any) => (
                  <View class="w-[150px]">
                    <Field label={f()[1]}>
                      <TextBox value={l()[f()[0]] || ""} onChange={(x: string) => set(i(), f()[0], x)} placeholder="0.00" />
                    </Field>
                  </View>
                )}
              </Each>
            </View>
          </View>
        )}
      </Each>
      <View class="flex-row items-center gap-2">
        <Btn label="+ Another employee" small onPress={() => lines.set([...lines(), { person: "" }])} />
        <View class="flex-1" />
        <Text class={`text-[12px] ${C.dim}`}>Tax withheld is what payroll recorded. Books does not compute it.</Text>
        <Btn label="Save draft" tone="go" onPress={save} />
      </View>
      <If when={Boolean(err())}>
        <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
      </If>
    </Panel>
  );
}
