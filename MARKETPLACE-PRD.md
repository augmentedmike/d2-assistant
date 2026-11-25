# Diablo II Offline Item Trading — Deferred Product Requirements

**Status:** Deferred concept; not part of the initial release
**Target game:** Original Diablo II + Lord of Destruction only
**Play mode:** Offline saves; no Battle.net or in-game online integration
**Purpose:** Explore a peer-to-peer way for players to list, inspect, and exchange equipment, runes, and gems using local save files.

## Product idea

Provide a private trading space inside D2 Assistant. Players can create offers from items in their bank, share an invitation with another player, inspect the offered items, agree on an exchange, and prepare verified `_d2a` character or stash copies for both sides.

The feature would connect users to exchange local item records. It would not connect to Diablo II, log into a game service, or conduct an in-game trade. Players continue using offline characters and manage the resulting save copies themselves.

## Intended users

- Diablo II / Lord of Destruction players who use offline character saves.
- Players who want a searchable way to offer and request surplus equipment, runes, gems, and charms.
- Players who want to exchange items without exposing entire character or stash files.

## Proposed workflow

1. Choose items from the bank and create a trade offer.
2. The offer shows the game version, item names, readable properties, rune/gem details, and any unknown decoded fields.
3. Share a private invite or trade code with a specific peer. Do not publish the user's save path or full save file.
4. The recipient reviews the offer and proposes selected items in return, or accepts a gift.
5. Both users review the final exchange and explicitly confirm it.
6. The app creates backups and prepares verified `_d2a` save copies for each user. Originals remain untouched.
7. Both users see a receipt with the transferred item fingerprints, output paths, and verification results.

## Requirements for a future implementation

- Trade only exact item records already present in a user's local bank/save; no item editor, generated items, or game modification.
- Keep the game format and item compatibility visible throughout the flow.
- Support requested items and simple offers for equipment, runes, and gems.
- Use explicit consent before sharing item data and before preparing output saves.
- Back up relevant local files before any write; write only `_d2a` copies and reparse/checksum-verify outputs.
- Make interrupted, rejected, expired, and completed offers unambiguous. Do not silently consume or export items.
- Use an authenticated peer session and encrypted transfer. Minimize any rendezvous service to connection setup; do not send full saves to a central service.
- Include abuse controls for unsolicited offers, spam, malicious payloads, and oversized data.
- Clearly label unknown properties and reject malformed/incompatible item records.

## Trust and duplication limits

This is an offline-save exchange, so the app has no authoritative game server to verify ownership or consume an item globally. A player can keep an old save backup, copy a save before trading, or use another editor. Item fingerprints can help identify the same nonzero-seed record across the app, but cannot prove uniqueness or ownership; simple items such as many runes and gems may not have a useful unique seed.

Therefore:

- Never promise that peer-to-peer trades are dupe-proof or authenticated by the game.
- The app can prevent accidental repeat exports within its own bank/transfer history, but cannot stop someone from restoring an older save outside the app.
- Before implementation, design and validate an honest bilateral handoff that removes traded items from the sender's newly prepared copy while preserving originals and clearly disclosing the backup limitation.
- If that experience cannot be made clear and safe, ship offer sharing only as a convenience and do not describe it as ownership transfer or escrow.

## Non-goals

- Diablo II: Resurrected support.
- Battle.net, online characters, online ladders, or in-game trade automation.
- Real-money trading, payments, fees, or an item price authority.
- Central custody of save files, accounts, or item inventory.
- Blockchain or claims of provable item scarcity.
- Guaranteed escrow, ownership, item provenance, or prevention of save copying.
- Public global listings at first; begin with invite-only peer exchanges if the trust model is approved.

## Open decisions for later

- Whether first release should be LAN-only, invite-code peer-to-peer over the internet, or both.
- How peers authenticate each other and establish a secure connection; peer discovery/signaling may still need a small service even if item data travels directly between peers.
- Whether an offer should support barter, gifts, or both.
- How item fingerprints handle identical simple items and cloned save files.
- Whether local trade receipts remain private or can be voluntarily shared as reputation evidence.
- What safeguards are needed before accepting untrusted item payloads.

## Release gate

Do not build or advertise this for the initial GitHub release. Revisit after the core bank/import/export flow is stable, real save-file fixtures are safe to publish, and the dupe/ownership limitations have a user-tested explanation.
