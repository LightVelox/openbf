"""Death-counter log compatibility; no game executable, data or sweep runs."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import sweep_seeds as sweep  # noqa: E402


LEGACY = (
    "deaths fixture: team0 kill 1 region 2 fall 3 vehicle 4 blast 5 "
    "floor 6 under 7 lost 8 stalled 9 | team1 kill 10 region 20 fall 30 "
    "vehicle 40 blast 50 floor 60 under 70 lost 80 stalled 90"
)
CURRENT = (
    "deaths fixture: team0 kill 1 region 2 fall 3 vehicle 4 blast 5 "
    "floor 6 under 7 lost 8 stalled 9 suicide 10 hero_exhausted 11 | "
    "team1 kill 101 region 102 fall 103 vehicle 104 blast 105 floor 106 "
    "under 107 lost 108 stalled 109 suicide 110 hero_exhausted 111"
)


class SweepDeaths(unittest.TestCase):
    def parse(self, line):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "seed.err"
            path.write_text(line + "\n")
            return sweep.parse_stderr(path)

    def test_historical_five_eight_and_nine_cause_rows_keep_team_positions(self):
        for width in [5, 8, 9]:
            with self.subTest(causes=width):
                line = LEGACY
                if width == 5:
                    line = line.replace(" floor 6 under 7 lost 8", "")
                    line = line.replace(" floor 60 under 70 lost 80", "")
                if width < 9:
                    line = line.replace(" stalled 90", "").replace(" stalled 9", "")
                parsed = self.parse(line)
                self.assertIn("fixture", parsed)
                deaths = parsed["fixture"]["deaths"]
                expected = [11, 22, 33, 44, 55, 66, 77, 88, 99][:width]
                self.assertEqual(deaths[:width], expected)
                self.assertTrue(all(value == 0 for value in deaths[width:]))

    def test_eleven_cause_rows_keep_both_teams_new_counters(self):
        parsed = self.parse(CURRENT)
        self.assertIn("fixture", parsed, "current death row must not be silently discarded")
        self.assertEqual(parsed["fixture"]["deaths"],
                         [102, 104, 106, 108, 110, 112, 114, 116, 118, 120, 122])

    def test_stalled_index_and_mixed_seed_aggregation_remain_distinct(self):
        self.assertEqual(sweep.STALLED, 8)
        stdout = {"fixture": dict(finished=True, winner=0, tick=90, stuck_max=0)}
        old, new = self.parse(LEGACY), self.parse(CURRENT)
        self.assertIn("fixture", new)
        stats = sweep.per_map({1: (stdout, old), 2: (stdout, new)})["fixture"]
        self.assertEqual(stats["deaths"],
                         [113, 126, 139, 152, 165, 178, 191, 204, 217, 120, 122])
        self.assertEqual(stats["deaths"][sweep.STALLED], 217)

    def test_world_total_and_labels_include_suicide_and_exhaustion(self):
        deaths = [100, 0, 0, 0, 0, 0, 0, 0, 17, 23, 31]
        self.assertEqual(sweep.fmt_deaths(deaths),
                         ("100", "71 (17 stalled, 23 suicide, 31 hero_exhausted)", "42 %"))

    def test_report_only_preserves_current_metrics_and_explains_the_new_causes(self):
        with tempfile.TemporaryDirectory() as temporary:
            out = Path(temporary)
            (out / "sweep_s1.txt").write_text("fixture team 0 wins at tick 90 stuck_max 0\n")
            (out / "sweep_s1.err").write_text(CURRENT + "\n")
            command = [sys.executable, str(Path(sweep.__file__)), "--out", str(out),
                       "--report-only", "--seeds", "1"]
            result = subprocess.run(command, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            report = (out / "seeds.md").read_text()
            self.assertIn("120 suicide", report)
            self.assertIn("122 hero_exhausted", report)
            self.assertIn("suicide = confirmed Respawn", report)
            self.assertIn("hero_exhausted = hero time drain with no credited attacker", report)


if __name__ == "__main__":
    unittest.main()
