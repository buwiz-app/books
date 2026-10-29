# Inline cards: one card, four classes

Status: in force for Books (2026-10-30). It replaces the inline-card part of the drawing "Books Inline Cards"
(research/drawings), which shows a Books title row and green buttons inside each card; that drawing is out of date
on those points.

## Why: what was wrong

A review of the 15 Books cards in a test Rox found these faults:

1. **One object, three wrappers.** Rox's row (title, "Mini app · Buwiz Books", Show/Hide), then Rox's frame header
   (title, maximize), then a Books title row. The reader passes the same title three times before a number.
2. **A toggle for something already shown.** Show/Hide serves Rox's consent rule (a third party's card does not
   run until you allow it) and folds old cards. Both belong to a chevron in the card's one header.
3. **Height guessed, not owned.** Each card app declared a size, measured itself, and Rox clamped the result.
   A tall list was cut at the bottom, and the bottom is where the buttons were. A short card was padded out.
4. **Actions inside the content.** On a decision card the button matters most, yet it moved with the content
   and was clipped with it.
5. **No shared anatomy.** Each card invented its layout: green links, green or violet buttons, tiles or rows.
   Rox already has a card language (approval, login and permission cards); Books built a second one.
6. **Three widths** (360, 480, the column), so cards did not line up with each other or with Rox's own.
7. **States as cards.** "Loading…" drew its own titled card; the 2307 card never left it when nothing was pending;
   errors showed raw tool text ("`person` is required").
8. **Noise.** Policy footnotes ("The payor's TIN stays in the BIR app") on cards. In a chat every line costs
   attention; policy belongs on the pages.

## The card

Rox draws the frame and the one header. Books draws only the body, in Rox's tokens (rox `app/src/cards/mod.rs`):

```
┌ Books · Draft to post                        ⟳  ⌄  ⤢ ┐  Rox: title, refresh, collapse, maximize
│ Oct 18 · Office supplies · Pixel Print Shop    Draft  │  Meta: when or what, a status pill. Never the title.
│ ₱ 1,350.00                                            │  the class's content
│ 67200 Office Supplies                      1,350.00   │  rows on hairlines; amounts mono, names not
│ 11010 Bank •••• 4021                       1,350.00   │  a list scrolls inside 240 points, without bounce
├───────────────────────────────────────────────────────┤
│ status or the last error        [Post] [Edit] [Discard]│  Foot: primary first, right-aligned; never scrolls
└───────────────────────────────────────────────────────┘
```

- Width 560 for every card: the agent column, the width of Rox's own cards.
- Padding 12, gap 8, text 13, notes 12 dim, buttons 28-point pills (primary ink on the surface, the rest in a
  well), as Rox's `shell`, `note` and `btn`.
- The card never scrolls and never bounces. Only a list scrolls, inside its own 240-point max, so the footer stays
  put and is never cut off.
- Height is the content's: the card reports it with `rox.fit`. Each class's parts are bounded, so it stays under
  the manifest's `max_height`. A body fills Rox's 96-point minimum: the footer sits at the bottom, and a settled
  line is centered.
- One card, one set of states: loading (skeleton in the body), empty (a sentence and where to go), error (the
  message and Try again), settled (one line with a check, after the action, as Rox's answered cards).

## The four classes

| Class | Anatomy | Books cards |
|---|---|---|
| **Glance** | Meta, one figure, at most a few rows; one "Open …" in the footer | cash, month, person |
| **List** | Meta, rows that scroll inside 240, a total; "Open …" in the footer | unpaid, P&L, person picker |
| **Decision** | Meta, what is decided (figure, facts, a choice); the buttons in the footer; settles to one line | draft, bank lines, mark paid, match the bank, close the month, 2307, for the BIR app |
| **Notice** | Meta, one sentence, one action | weekly tidy-up, month end, BIR hand-off |

Rules for Decision cards: a choice the action depends on (a category, the account the money moved through) is
picked in the body and named in the primary button ("Post as Utilities"); a money account is never a silent
default. Row-level actions (a return's "It's correct") stay on their row.

## What Rox owns (asked of the rox session, 2026-10-30)

1. One card: the Show/Hide row folds into the frame header as a chevron; collapsed means the header only. A card
   that does not auto-mount starts collapsed, and expanding it is the consent to mount (9.7).
2. The frame never scrolls; a wheel over a card whose content cannot scroll reaches the chat (nsh events.rs).
3. Inline width up to the column (560).
4. Proposed: `inline.class` in the manifest (height ranges by class), `rox.actions([...])` so Rox draws a decision
   card's footer with its own `btn`, `rox.settle(text)` to collapse an answered card to `outcome_line`, and
   `rox.meta({text, pill})` for a status in the header. Until then Books mirrors Rox's tokens itself.

## Pages: the same review, still to do

- **Today**: two ways to the same place (the header's + Money in/out and the Quick tiles; Quick's Ledger and
  Unpaid repeat the sidebar). The "also in chat as /books" hint is noise.
- **Ledger**: the solid green filter bar is the loudest thing on the page for the least important fact.
- **Reconcile**: red "To do" in every cell reads as errors; to-do is neutral, problems are red.
- **Forms**: the densest page. The owner counters, the return banner and the cross-checks compete with the one
  action that matters (Hand to BIR draft), which is small and on the right. It needs a step order: confirm the
  return, check the totals, hand over.
- **Accounts**: some 60 rows, most at zero; show the accounts in use first.
- Everywhere: violet BIR footnotes on many pages, and 11-point grey text that is hard to read; 12 is the floor.
