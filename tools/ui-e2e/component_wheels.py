import base64
import configparser
import csv
import fnmatch
import hashlib
import io
import tomllib
import zipfile
from email.parser import BytesParser

from packaging.markers import Marker
from packaging.requirements import Requirement
from packaging.specifiers import SpecifierSet
from packaging.utils import canonicalize_name


def requirement(value):
    entry = Requirement(value)
    return canonicalize_name(entry.name), tuple(sorted(entry.extras)), str(entry.specifier), str(entry.marker or "")


def configuration(source, product):
    value = tomllib.loads((source / "pyproject.toml").read_text(encoding="utf-8"))
    project = value["project"]
    backend = value["build-system"]
    if canonicalize_name(project["name"]) != product or project.get("dynamic"):
        raise ValueError("Component source metadata identity differs")
    if (backend["build-backend"] != "setuptools.build_meta"
            or "backend-path" in backend
            or any(canonicalize_name(Requirement(entry).name) not in {"setuptools", "wheel"}
                   for entry in backend["requires"])
            or any((source / name).exists() for name in ("setup.py", "setup.cfg"))
            or "cmdclass" in value.get("tool", {}).get("setuptools", {})):
        raise ValueError("Only the static setuptools component backend is supported")
    return value


def entry_points(project, content):
    expected = dict(project.get("entry-points", {}))
    if any(group in expected for group in ("console_scripts", "gui_scripts")):
        raise ValueError("Declare console and GUI scripts in their project tables")
    for field, group in (("scripts", "console_scripts"), ("gui-scripts", "gui_scripts")):
        if project.get(field):
            expected[group] = project[field]
    entries = configparser.ConfigParser(interpolation=None)
    entries.optionxform = str
    try:
        entries.read_string(content.decode("utf-8"))
    except configparser.Error as error:
        raise ValueError("Wheel entry points differ from pinned source") from error
    actual = {group: dict(entries[group]) for group in entries.sections()}
    if entries.defaults() or actual != expected:
        raise ValueError("Wheel entry points differ from pinned source")


def source_payload(source, configuration):
    expected = {}
    settings = configuration["tool"]["setuptools"]
    for module in settings.get("py-modules", []):
        file = source / (module.replace(".", "/") + ".py")
        expected[file.relative_to(source).as_posix()] = file.read_bytes()
    discovery = settings.get("packages", {}).get("find", {})
    for location in discovery.get("where", ["."]):
        base = source / location
        for initializer in base.rglob("__init__.py"):
            folder = initializer.parent
            package = folder.relative_to(base).as_posix().replace("/", ".")
            if not any(fnmatch.fnmatchcase(package, pattern) for pattern in discovery.get("include", ["*"])):
                continue
            if any(fnmatch.fnmatchcase(package, pattern) for pattern in discovery.get("exclude", [])):
                continue
            for file in folder.glob("*.py"):
                expected[file.relative_to(base).as_posix()] = file.read_bytes()
            for owner, patterns in settings.get("package-data", {}).items():
                if fnmatch.fnmatchcase(package, owner):
                    for pattern in patterns:
                        for file in folder.glob(pattern):
                            if file.is_file():
                                expected[file.relative_to(base).as_posix()] = file.read_bytes()
    project = configuration["project"]
    distribution = canonicalize_name(project["name"]).replace("-", "_") + "-" + project["version"]
    for destination, patterns in settings.get("data-files", {}).items():
        for pattern in patterns:
            for file in source.glob(pattern):
                expected[f"{distribution}.data/data/{destination}/{file.name}"] = file.read_bytes()
    if not expected:
        raise ValueError("No declared component package payload")
    return expected


def validate_wheel(source, wheel, product):
    settings = configuration(source, product)
    project = settings["project"]
    distribution = product.replace("-", "_") + "-" + project["version"]
    if wheel.name != distribution + "-py3-none-any.whl":
        raise ValueError("Wheel filename or platform identity differs")
    with zipfile.ZipFile(wheel) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)) or any(
            name.startswith("/") or "\\" in name or ".." in name.split("/") for name in names
        ):
            raise ValueError("Unsafe or duplicate wheel entry")
        if sum(entry.file_size for entry in archive.infolist()) > 64 * 1024 * 1024:
            raise ValueError("Wheel expanded payload exceeds the component bound")
        files = {name: archive.read(name) for name in names if not name.endswith("/")}
    prefix = distribution + ".dist-info/"
    wheel_metadata = BytesParser().parsebytes(files[prefix + "WHEEL"])
    if wheel_metadata["Root-Is-Purelib"] != "true" or "py3-none-any" not in wheel_metadata.get_all("Tag", []):
        raise ValueError("Wheel platform metadata differs from the portable component contract")
    metadata = BytesParser().parsebytes(files[prefix + "METADATA"])
    if canonicalize_name(metadata["Name"]) != product or metadata["Version"] != project["version"]:
        raise ValueError("Wheel metadata identity differs from pinned source")
    if SpecifierSet(metadata["Requires-Python"]) != SpecifierSet(project["requires-python"]):
        raise ValueError("Wheel Python requirement differs from pinned source")
    dependencies = list(project.get("dependencies", []))
    for extra, entries in project.get("optional-dependencies", {}).items():
        for value in entries:
            entry = Requirement(value)
            entry.marker = Marker(f'({entry.marker}) and extra == "{extra}"' if entry.marker else f'extra == "{extra}"')
            dependencies.append(str(entry))
    if sorted(map(requirement, metadata.get_all("Requires-Dist", []))) != sorted(map(requirement, dependencies)):
        raise ValueError("Wheel dependency metadata differs from pinned source")
    expected = source_payload(source, settings)
    if expected != {name: data for name, data in files.items() if not name.startswith(prefix)}:
        raise ValueError("Wheel package payload differs from pinned source")
    entry_points(project, files.get(prefix + "entry_points.txt", b""))
    rows = list(csv.reader(io.StringIO(files[prefix + "RECORD"].decode("utf-8"))))
    if len(rows) != len(files) or {row[0] for row in rows} != set(files):
        raise ValueError("Wheel RECORD does not cover every file")
    for name, encoded, size in rows:
        if name == prefix + "RECORD":
            if encoded or size:
                raise ValueError("Wheel RECORD self entry differs")
        else:
            sha = base64.urlsafe_b64encode(hashlib.sha256(files[name]).digest()).decode().rstrip("=")
            if encoded != "sha256=" + sha or size != str(len(files[name])):
                raise ValueError("Wheel RECORD integrity differs")
    return project
