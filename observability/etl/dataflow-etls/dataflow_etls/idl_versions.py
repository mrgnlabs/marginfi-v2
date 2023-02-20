import glob
import os
from pathlib import Path
from typing import List, Literal, Tuple, Optional
from anchorpy import Program, Provider
from anchorpy_core.idl import Idl
from solders.pubkey import Pubkey

Cluster = Literal["devnet", "mainnet"]
IdlBoundary = tuple[int, int]
ProgramIdlBoundaries = dict[str, List[IdlBoundary]]
ClusterIdlBoundaries = dict[Cluster, ProgramIdlBoundaries]


class VersionedProgram(Program):
    version: str
    cluster: Cluster

    def __init__(self, cluster: Cluster, version: str, idl: Idl, program_id: Pubkey,
                 provider: Optional[Provider] = None):
        self.version = version
        self.cluster = cluster
        super(VersionedProgram, self).__init__(idl, program_id, provider)


class VersionedIdl:
    VERSIONS: ClusterIdlBoundaries = {"devnet": {
        "A7vUDErNPCTt9qrB6SSM4F6GkxzUe9d8P3cXSmRg4eY4": [(196494976, 0), (196520454, 1)]
    }}

    @staticmethod
    def get_idl_for_slot(cluster: Cluster, program_id: str, slot: int) -> Tuple[Idl, str]:
        idl_boundaries = VersionedIdl.VERSIONS[cluster][program_id]

        idl_version = None
        for boundary_slot, version in idl_boundaries:
            # todo: returns latest for upgrade slot, can throw if tx executed in same slot, before upgrade
            if boundary_slot > slot:
                idl_version = f"v{version}"
                break

        if idl_version is None:
            sorted_idls = [os.path.basename(path).removesuffix(".json").removeprefix("marginfi-") for path in
                           glob.glob(f"idls/{cluster}/marginfi-v*.json")]
            sorted_idls.sort()
            idl_version = sorted_idls[-1]

        path = Path(f"idls/{cluster}/marginfi-{idl_version}.json")
        raw = path.read_text()
        idl = Idl.from_json(raw)

        return idl, idl_version
