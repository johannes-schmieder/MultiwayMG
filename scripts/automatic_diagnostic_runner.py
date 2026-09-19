"""Preserved isolated processes for the separate diagnostic protocol."""
import json,os,signal,subprocess,time
from automatic_diagnostic_protocol import parse_output,check_diagnostic
from automatic_recipes import case_id
import prepared_serial as base
from validate_prepared_automatic import check_probe
def run_one(binary,data,policy,system,out,index,spec,origin,env):
    command=['/usr/bin/time','-l' if system=='Darwin' else '-v',str(binary),spec['route'],spec['layout']]
    started=time.perf_counter_ns();error=None
    exit_code=None;failure_status=None
    try:
        child=subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
            env=env,start_new_session=True)
    except OSError as exc:
        stdout=b'';stderr=str(exc).encode();error=str(exc);failure_status='launch_error'
    else:
        try:stdout,stderr=child.communicate(data,timeout=policy['timeout_seconds'])
        except subprocess.TimeoutExpired:
            try:os.killpg(child.pid,signal.SIGKILL)
            except ProcessLookupError:pass
            stdout,stderr=child.communicate();error='declared isolated process timeout';failure_status='timeout'
        exit_code=child.returncode
    ended=time.perf_counter_ns()
    row=dict(spec,index=index,case_id=case_id(spec['case']),input_sha256=base.sha(data),
        start_ns=started-origin,end_ns=ended-origin,process_wall_ns=ended-started,exit_code=exit_code,
        stdout_path=f'{index:06d}.stdout',stderr_path=f'{index:06d}.stderr',
        stdout_sha256=base.sha(stdout),stderr_sha256=base.sha(stderr))
    (out/row['stdout_path']).write_bytes(stdout);(out/row['stderr_path']).write_bytes(stderr)
    if error:row.update(status=failure_status,error=error,resource_status='unavailable_after_timeout' if failure_status=='timeout' else 'unavailable_after_launch_error')
    else:
        try:
            probe=parse_output(stdout.decode());resource=base.resources(stderr.decode(),system)
            json.dumps([probe,resource],allow_nan=False)
            row.update(status='returned',probe=probe,resources=resource,resource_status='measured')
        except (ValueError,UnicodeError) as exc:
            row.update(status='protocol_error',error=str(exc),resource_status='unavailable_or_invalid_protocol')
    return row

def check_reference(row, policy):
    p=row['probe']
    check_diagnostic(p,row['layout'],False,row['process_wall_ns'])
    # The unchanged numerical/cost contract is schema1. This checked projection
    # validates only that common boundary; all new fields are checked above.
    return check_common(row,policy)

def check_common(row,policy):
    # Only reuse the numerical/outer-cost checks. The caller validates the real
    # profiling flag first; this diagnostic protocol never reports speedups.
    return check_probe(dict(row['probe'],schema=1,profiling=False),row,policy)
