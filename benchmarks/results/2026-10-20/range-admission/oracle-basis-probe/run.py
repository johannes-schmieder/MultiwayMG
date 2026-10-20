"""Run the oracle-basis probe on every unit-weight recipe topology (seed 10001)."""
import json, subprocess, sys
sys.path.insert(0, 'scripts')
from automatic_recipes import FAMILIES, SHAPES, generate
policy = json.load(open('benchmarks/policies/prepared-automatic-v1.json'))
binary = sys.argv[1]
for profile in ('smoke', 'development'):
    for family in FAMILIES:
        for shape in SHAPES:
            data, _ = generate(policy, profile, dict(family=family, shape=shape, weights='unit',
                                                     seed=10001, width=1))
            out = subprocess.run([binary], input=data, capture_output=True, check=True)
            print(profile, family, shape, ' | '.join(out.stdout.decode().strip().splitlines()))
