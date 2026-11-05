# Watch log analysis

Reads Caravan's `bot.watch` lines (a training party seen by a standing observer; the format in
Caravan's `docs/watch-log.md`, the requirements in `CARAVAN_LOGGER_REQUIREMENTS.md`) and
produces the numbers the Training Field is measured against (`PLAN.md` Findings, "The watch
log"). Standard library only.

Keep every slice and every table outside git: they hold other players' names.

```
# One window straight from the log (reads the whole file once, 10-20 s), or a slice cut once:
grep 'event="bot.watch"' ~/.caravan/caravand.log | awk '$1>="2026-11-05T18:15:28" && $1<"2026-11-05T18:38:16"' > /path/outside/git/second.log

uv run --no-project scripts/watchlog/checks.py  --slice /path/outside/git/second.log   # section 7
uv run --no-project scripts/watchlog/measure.py --slice /path/outside/git/second.log   # section 5
uv run --no-project scripts/watchlog/export.py  --slice /path/outside/git/second.log --out /path/outside/git/tables
# or: --log ~/.caravan/caravand.log --account babysock92 --from 2026-11-05T18:15:28 --to 2026-11-05T18:38:16
```

`watchlog.py` parses the lines, replays them into entities and rebuilds every track from the
spawn legs, walks, speed and motion changes, position corrections, teleport landings and
knockbacks (the Traps of `docs/watch-log.md`); `merge` joins the records of an entity that left
sight and came back. The roles come from the `session` line; the observer is the origin of
nothing (the centre is the dps' median position); stretches where the observer is attacked are
cut from every time-sampled measure.
