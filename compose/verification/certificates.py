"""Create an ephemeral private CA and local-only server certificate."""

import pathlib
import shutil
import subprocess


def openssl(*arguments):
    subprocess.run(
        ["openssl", *arguments],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


directory = pathlib.Path("/certificates")
openssl(
    "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
    "-keyout", str(directory / "ca.key"), "-out", str(directory / "ca.crt"),
    "-subj", "/CN=Ephemeral production acceptance CA",
    "-addext", "basicConstraints=critical,CA:TRUE",
    "-addext", "keyUsage=critical,keyCertSign,cRLSign",
)
openssl(
    "req", "-newkey", "rsa:2048", "-nodes",
    "-keyout", str(directory / "server.key"), "-out", str(directory / "server.csr"),
    "-subj", "/CN=api",
)
extensions = directory / "server.ext"
extensions.write_text(
    "subjectAltName=DNS:api,DNS:mailpit,DNS:localhost,IP:127.0.0.1\n"
    "basicConstraints=critical,CA:FALSE\n"
    "keyUsage=critical,digitalSignature,keyEncipherment\n"
    "extendedKeyUsage=serverAuth\n",
    encoding="utf-8",
)
openssl(
    "x509", "-req", "-in", str(directory / "server.csr"),
    "-CA", str(directory / "ca.crt"), "-CAkey", str(directory / "ca.key"),
    "-CAcreateserial", "-days", "2", "-out", str(directory / "server.crt"),
    "-extfile", str(extensions),
)
(directory / "server.key").chmod(0o644)
shutil.copyfile(directory / "ca.crt", "/certificate-authorities/ca.crt")
(directory / "ca.key").unlink()
print("Ephemeral CA and verified-hostname certificates generated.")
