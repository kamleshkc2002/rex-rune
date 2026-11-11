"""A private sshd for the acceptance scripts: a high port on 127.0.0.1, throwaway keys, an
isolated config, no root and nothing in ~/.ssh. Shared by remote-control-acceptance.py and
download-acceptance.py; not a script of its own.

start(work) writes host-key, client-key(.pub), sshd_config, ssh_config and a `bin/ssh` wrapper
(an `ssh` that always uses the private client config) under `work`, starts sshd, proves a
login, and returns (process, port, user). When sshd cannot run as this user it returns a
string that says why instead. The caller puts `work/bin` first on the client's PATH and stops
the process by PID (see stop())."""
import os
from pathlib import Path
import pwd
import shlex
import shutil
import socket
import subprocess
import time


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def start(work):
    work = Path(work)
    sshd, ssh, keygen = (shutil.which(tool) for tool in ("sshd", "ssh", "ssh-keygen"))
    if not all((sshd, ssh, keygen)):
        return "sshd/ssh/ssh-keygen not available"
    for name in ("host-key", "client-key"):
        subprocess.run([keygen, "-q", "-t", "ed25519", "-N", "", "-f", str(work / name)], check=True)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    user = pwd.getpwuid(os.getuid()).pw_name
    server_config = work / "sshd_config"
    server_config.write_text(f"""ListenAddress 127.0.0.1
Port {port}
HostKey {work}/host-key
PidFile {work}/sshd.pid
AuthorizedKeysFile {work}/client-key.pub
StrictModes no
UsePAM no
PasswordAuthentication no
KbdInteractiveAuthentication no
AllowUsers {user}
AllowTcpForwarding yes
AllowStreamLocalForwarding yes
""")
    client_config = work / "ssh_config"
    client_config.write_text(f"""Host *
  IdentityFile {work}/client-key
  IdentitiesOnly yes
  UserKnownHostsFile {work}/known_hosts
  GlobalKnownHostsFile /dev/null
  StrictHostKeyChecking accept-new
  BatchMode yes
  ConnectTimeout 2
""")
    server = subprocess.Popen([sshd, "-D", "-e", "-f", str(server_config)],
                              stdout=(work / "sshd.log").open("w"), stderr=subprocess.STDOUT)
    time.sleep(.4)
    test = subprocess.run([ssh, "-F", str(client_config), "-p", str(port), f"{user}@127.0.0.1", "true"],
                          capture_output=True, text=True, timeout=5)
    if test.returncode:
        (work / "ssh-check.log").write_text(test.stderr)
        stop(server)
        return "private sshd cannot authenticate; see ssh-check.log and sshd.log"
    (work / "bin").mkdir(exist_ok=True)
    (work / "bin/ssh").write_text(f"#!/bin/sh\nexec {shlex.quote(ssh)} -F {shlex.quote(str(client_config))} \"$@\"\n")
    (work / "bin/ssh").chmod(0o755)
    return server, port, user
