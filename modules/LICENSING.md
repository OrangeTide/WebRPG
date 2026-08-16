# Licensing of the bundled modules

The rest of this repository is MIT-0. **This directory is not.** The modules
carry other people's work under licences that impose conditions MIT-0 does not,
and two of them cannot be relicensed at all:

| Module | Source work | Licence | Conditions that follow the content |
|--------|-------------|---------|------------------------------------|
| `tunnel-goons` | Tunnel Goons, Nate Treme (Highland Paranormal Society) | Creative Commons 4.0 International | Credit the author, name and link the licence, say what was changed |
| `sky-blind-spire` | The Sky-Blind Spire, Michael Prescott, 2016 | [CC BY-NC 3.0](https://creativecommons.org/licenses/by-nc/3.0/) | Attribution, **non-commercial use only**, identify it as an adaptation |
| `cairn-spellbooks` | Cairn, Yochai Gal and contributors | [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) | Attribution, indicate changes, and **share adaptations under CC BY-SA 4.0** |

If you are reusing any of this, the module's licence governs, not the
repository's. In particular you cannot take the Sky-Blind Spire module into a
commercial product, and anything you build on the Cairn spell lists has to stay
CC BY-SA 4.0.

## tunnel-goons

Tunnel Goons is by Nate Treme, Highland Paranormal Society, released under a
Creative Commons 4.0 International licence: <https://natetreme.itch.io/tunnelgoons>
grants the right to share and to adapt the material, including commercially,
which carries the usual attribution condition. The SRD is at
<https://tunnelgoons.com/srd>.

**What this module takes:** the roll model (2d6 + ability + item against a
Difficulty Score), the three ability names, the difficulty ladder, and the three
character-creation tables (Childhood, Profession, During the War), transcribed
into `rules.json`, `sheet.json`, and `tables.json`.

**What was changed:** the tables are transcribed as data rather than prose; the
sheet schema, creature schema, and help page are written for this software. The
borrowed rules documented in the help page (spellbooks, scrolls, Fatigue, gear
sacrifice, gear tags) are **not** from Tunnel Goons; see below.

## sky-blind-spire

The Sky-Blind Spire is © 2016 Michael Prescott, licensed
[CC BY-NC 3.0](https://creativecommons.org/licenses/by-nc/3.0/), published at
<https://blog.trilemma.com/2016/04/the-sky-blind-spire.html>.

**What this module takes:** the adventure's premise, its 24 numbered rooms and
their contents, the factions, the named treasures, and the creatures, adapted
to Tunnel Goons.

**What was changed:** this is an adaptation. Room text was rewritten for
play at a virtual table, monsters were restated as Difficulty Scores, the room
cards were written so that nothing blue is named, and rulings were added that
the one-page original does not carry (the traverse, the undines' reach, the
spellbook shelf). Two NPCs, Squeaks and Lady Beatrice, and the loot tables are
original to this material and are not Prescott's.

**Non-commercial:** the licence forbids commercial use of this module and of
anything derived from it. The adventure's art is deliberately not committed for
the same reason; `scripts/fetch-spire-assets.sh` downloads it for personal use.

## cairn-spellbooks

Cairn is by Yochai Gal and contributors; its text is licensed
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). Sources:
<https://cairnrpg.com/second-edition/wardens-guide/spellbooks/> (the core 100)
and <https://cairnrpg.com/resources/more-spellbooks/> (the community 216). The
latter page credits Chris McDowall, Mike Evans, Goblin Punch's GLOG, and
several SRDs as its own antecedents.

**What this module takes:** both spell lists, name and description, essentially
verbatim.

**What was changed:** the two lists are kept separate and presented as a d100
and a d666 table; five misspellings in the community list were corrected
(Skillful, Doppelganger, Fiery, Otherworldly, Psychic); item cards are generated
from both lists and deduplicated by name, with the Warden's Guide wording
preferred where a spell appears in both.

**ShareAlike:** this module's data files are licensed CC BY-SA 4.0, as the
licence requires. That is a condition on the content, not a choice.

## Not licensed content

Some things named in these modules are referenced rather than reproduced, and
carry no licence obligation here:

- **Knave**, Ben Milton — the inventory-slot rule and the idea of dealing gear
  as cards. Named because the borrowing should be visible; no text is taken.
- **Mörk Borg** — cited only as a parallel for one-use scrolls.
- **Wizards & Lizards** (the Pigton playset) and **Bleakmire Marches** — Jon
  Mayo's own works in progress, the source of the arms, armour, snares, kit, and
  relics in the Spire item deck, and of the gear tags.

Rules and mechanics are not copyrightable; the expression of them is. Where this
repository reproduces someone's wording, it says so above.

## Everything else

Text written for these modules by this project, the card art in
`tunnel-goons/cards/`, and the scripts that build them are PUBLIC DOMAIN
(CC0-1.0), except where that would conflict with a ShareAlike obligation above.
