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


if __name__ == "__main__":
    unittest.main()
