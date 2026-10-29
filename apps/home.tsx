// Buwiz Books home (P0 stub): a neutral label and the setup status with a
// count of books. No amounts, TINs or names: inline cards from a
// first-party publisher mount without a click (RK-21), so this card shows
// counts and status only. Imports only nsh and rox (spec 6.3); data comes
// from rox.call alone. Every tool called here is in permissions.tools.
import { signal, Text, View } from "nsh";
import { rox } from "rox";

let sent = 0;

// Report the content's own height so an inline card fits it (rox.fit).
const fit = (e: any) => {
  const h = Math.ceil(e.bounds.size.height);

  if (Math.abs(h - sent) >= 1 && typeof rox.fit === "function") rox.fit((sent = h));
};

const STATUS_TEXT: Record<string, string> = {
  not_set_up: "Not set up yet",
  ready: "Ready",
};

export default function Home() {
  const line = signal("Checking…");

  rox.call("books_status", {}).then(
    (r: any) => {
      const status = STATUS_TEXT[String(r.status)] ?? "Unknown status";
      const books = String(r.books) === "1" ? "1 book" : `${String(r.books)} books`;

      line.set(`${status} · ${books}`);
    },
    (e: any) => line.set(`Status unavailable (${String(e.code)})`),
  );

  return (
    <View class="p-3 gap-1 bg-white dark:bg-[#161616]" onLayout={fit}>
      <Text class="text-[14px] text-[#1c1c1c] dark:text-[#f5f5f5]">Books</Text>
      <Text class="text-[12px] text-[#1c1c1c99] dark:text-[#f5f5f599]">{line()}</Text>
    </View>
  );
}
