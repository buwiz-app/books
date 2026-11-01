// "What is this document?" (evidence-4): the assistant's proposed kind for one Books document, as a generic kind,
// a whole-number confidence and a link count. Props are opaque tickets and a mode only. At most three buttons:
// Accept, Discard, Preview; once decided, one button opens the matching Books page. Nothing here creates an entry,
// an unpaid item, statement lines or a certificate: the page it opens is where the user does that.

const DOC_KIND: Record<string, string> = {
  statement: "Bank statement",
  bill: "Bill",
  invoice: "Invoice",
  receipt: "Receipt",
  payslip: "Payslip",
  contract: "Contract",
  tax_form: "Tax form",
  other: "Other document",
};
const DOC_ROUTE: Record<string, [string, string]> = {
  ledger_draft: ["ledger", "Open the Ledger"],
  unpaid_review: ["unpaid", "Open Unpaid"],
  statement_import: ["reconcile", "Import the statement"],
  certificate_review: ["certificates", "Open Certificates"],
};

function docPreview() {
  if (!buwiz.preview) return toast("Previews need a newer Buwiz Bot.");
  buwiz.preview({ tool: "books_app_evidence_preview", args: { evidence: CP.evidence } }).catch((e: Any) => toast(errText(e)));
}

function DocumentCard() {
  const r = read(
    () => ({ v: ver() }),
    () => buwiz.call("books_app_evidence_review", { proposal: CP.proposal }).then((l: Any) => (l.rows || [])[0] || null),
  );
  const d = () => r() as Any;
  const kind = () => DOC_KIND[String(d()?.accepted_kind || d()?.kind || "")] || "Document";
  const route = () => DOC_ROUTE[String(d()?.route || "")];
  return (
    <View>
      <Wait res={r} />
      <If when={r() === null}>
        <Empty text="That document is not in Books." sub="Add it in Books documents first." />
      </If>
      <If when={Boolean(d()) && d()?.status === "discarded"}>
        <Settled text="Proposal discarded" sub="The document keeps its kind." />
      </If>
      <If when={Boolean(d()) && d()?.status === "accepted"}>
        <Body
          foot={
            <Foot>
              <If when={Boolean(route())}>
                <Act label={route() ? route()[1] : ""} primary onPress={() => buwiz.open({ app: "books", props: { page: route()[0] } }).catch(() => null)} />
              </If>
            </Foot>
          }
        >
          <Settled text={`Saved as ${kind().toLowerCase()}`} sub={route() ? "Nothing is recorded until you review it in Books." : "Kept and previewable in Books."} />
        </Body>
      </If>
      <If when={Boolean(d()) && d()?.status === "draft"}>
        <Body
          foot={
            <Foot>
              <Act label="Accept" primary onPress={() => act("books_app_evidence_kind", { proposal: CP.proposal, decision: "accept" }).catch(() => null)} />
              <Act label="Discard" onPress={() => act("books_app_evidence_kind", { proposal: CP.proposal, decision: "discard" }).catch(() => null)} />
              <Act label="Preview" onPress={docPreview} />
            </Foot>
          }
        >
          <Meta text="Proposed by the assistant" right={<Pill tone="draft" label="Draft" />} />
          <Hero text={kind()} />
          <View>
            <Row label="Confidence" text={`${String(d()?.confidence ?? "")} of 100`} />
            <Row label="Linked" text={`${String(d()?.links ?? 0)} records`} />
          </View>
        </Body>
      </If>
    </View>
  );
}

export default function Document() {
  return (
    <Scroll class={`flex-1 ${K.surface}`} bounces={false} scrollable={false}>
      <DocumentCard />
    </Scroll>
  );
}
