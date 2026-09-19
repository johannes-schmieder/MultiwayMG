"""Adversarial checks for complete fixed-work microbenchmark records."""
import json,unittest
from prepared_serial import ROOT
from dense_reconstruction import POLICY,validate_stdout

def fixture():
    p=json.loads((ROOT/POLICY).read_text());lines=[]
    for case,n in enumerate(p['dimensions']):
        for round in range(6):
            for pos in range(2):
                arm=p['arms'][(case+round+pos)%2];kind='warmup' if round==0 else 'measure';i=p['operation_budget']//(2*n*n)
                lines.append(f'dense_reconstruction\t1\t{n}\t{arm}\t{max(0,round-1)}\t{kind}\t{i}\t{100 if arm=="old" else 50}\t0123456789abcdef\tfedcba9876543210')
    return p,lines

class DenseReconstructionTests(unittest.TestCase):
    def test_complete_pairs_and_declared_cost_scope(self):
        p,lines=fixture();r=validate_stdout('\n'.join(lines),p)
        self.assertEqual((r['samples'],r['measured_samples']),(192,160));self.assertAlmostEqual(r['balanced_geomean'],2)
        self.assertFalse(r['competitive_qualification']);self.assertFalse(r['default_selected'])
    def test_missing_duplicate_reordered_and_wrong_work_fail(self):
        for mode in ['missing','duplicate','reordered','work']:
            p,lines=fixture()
            if mode=='missing':lines.pop(0)
            elif mode=='duplicate':lines.append(lines[0])
            elif mode=='reordered':lines[0],lines[1]=lines[1],lines[0]
            else:
                f=lines[0].split('\t');f[6]=str(int(f[6])+1);lines[0]='\t'.join(f)
            with self.subTest(mode=mode),self.assertRaises(ValueError):validate_stdout('\n'.join(lines),p)
    def test_every_warmup_and_measurement_requires_exact_math(self):
        for row in [0,1,2,3,191]:
            for field in [8,9]:
                p,lines=fixture();f=lines[row].split('\t');f[field]='0'*16;lines[row]='\t'.join(f)
                with self.subTest(row=row,field=field),self.assertRaises(ValueError):validate_stdout('\n'.join(lines),p)
    def test_zero_nonfinite_malformed_and_unknown_records_fail(self):
        for field,value in [(7,'0'),(7,'NaN'),(7,'-1'),(7,'1.0'),(1,'2'),(8,'false'),(3,'unknown')]:
            p,lines=fixture();f=lines[0].split('\t');f[field]=value;lines[0]='\t'.join(f)
            with self.subTest(field=field,value=value),self.assertRaises(ValueError):validate_stdout('\n'.join(lines),p)

if __name__=='__main__':unittest.main()
