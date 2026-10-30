"""python3 -m unittest discover -s scripts/tests   (human_profile: lives from a match log)"""
import os, sys, unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import human_profile as hp  # noqa: E402

HEAD = {"kind": "header", "unit": 0, "team": 0}


class Lives(unittest.TestCase):
    def rows(self, class_change):
        r = [["spawn", 10, 0, 0], ["shot", 12, 1], ["hit", 13, 0, 5, 1, 1, 60.0], ["kill", 14, 0, 5, 1]]
        if class_change:
            r.append(["spawn", 20, 3, 0])  # a class change: the generation moves while alive
        r += [["death", 30, 0], ["spawn", 40, 0, 0], ["shot", 45, 1]]
        return r

    def test_a_class_change_is_not_a_new_life(self):
        t = hp.tally(HEAD, self.rows(True), {"kind": "end", "tick": 50})
        self.assertEqual(len(t["lives"]), 2)
        self.assertEqual(t["alive_ticks"], 30)
        self.assertEqual((t["shots"], t["kills"], t["deaths"], t["hits"]), (2, 1, 1, 1))

    def test_respawns_only(self):
        t = hp.tally(HEAD, self.rows(False), {"kind": "end", "tick": 50})
        self.assertEqual(len(t["lives"]), 2)
        self.assertEqual(t["alive_ticks"], 30)
        self.assertEqual((t["shots"], t["kills"], t["deaths"], t["hits"]), (2, 1, 1, 1))


class Calibration(unittest.TestCase):
    def test_rounds_play_16_per_side_on_mission_tickets(self):
        # The tiers and the proxy were tuned at 16 per side; the sweep's default is now the map's own sizes.
        from unittest import mock
        seen = {}
        with mock.patch.object(hp.subprocess, "run", side_effect=lambda cmd, env, **kw: seen.update(env)), \
                mock.patch.dict(os.environ, {}, clear=False):
            for k in ("SWEEP_COMBATANTS", "SWEEP_MISSION_TICKETS", "SWEEP_NOFILL"):
                os.environ.pop(k, None)
            hp.run_round("bin", {"team": 0, "map": "bf1_nab2"}, "good", {}, 1, "/nonexistent")
        self.assertEqual(seen["SWEEP_COMBATANTS"], "16")
        self.assertEqual(seen["SWEEP_MISSION_TICKETS"], "1")
        self.assertNotIn("SWEEP_NOFILL", seen)


if __name__ == "__main__":
    unittest.main()
