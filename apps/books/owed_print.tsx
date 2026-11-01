// Print preview of one owed document (buwiz.preview). Books issues no BIR
// documents, so the page is a "Statement of account" or a "Bill record".
// The method is absent without the preview permission or host support.

function OwedPrintBtn(p: { ticket: string; side: string }) {
  const open = () => {
    buwiz
      .preview({ tool: "books_owed_document", args: { owed: p.ticket }, title: p.side === "collect" ? "Statement of account" : "Bill record" })
      .catch((e: Any) => toast(e && e.code === "not_supported" ? "Print preview needs Buwiz Bot on macOS." : "Could not open the print preview."));
  };
  return (
    <If when={typeof buwiz.preview === "function"}>
      <Btn label="Print" small onPress={open} />
    </If>
  );
}
