import glob
import os
from pathlib import Path
from typing import List, Literal, Tuple, Optional, Dict
from anchorpy import Program, Provider, Wallet
from anchorpy.utils.rpc import AsyncClient
from anchorpy_core.idl import Idl
from solders.pubkey import Pubkey

Cluster = Literal["devnet", "mainnet"]
IdlBoundary = tuple[int, int]
ProgramIdlBoundaries = dict[str, List[IdlBoundary]]
ClusterIdlBoundaries = dict[Cluster, ProgramIdlBoundaries]


class VersionedProgram(Program):
    version: int
    cluster: Cluster

    def __init__(self, cluster: Cluster, version: int, idl: Idl, program_id: Pubkey,
                 provider: Optional[Provider] = None):
        self.version = version
        self.cluster = cluster
        super(VersionedProgram, self).__init__(idl, program_id,
                                               provider or Provider(AsyncClient("http://localhost:8899"),
                                                                    Wallet.dummy()))


# /!\ Boundaries need to be ordered /!\
IDL_VERSIONS: ClusterIdlBoundaries = {
    "devnet": {
        # "A7vUDErNPCTt9qrB6SSM4F6GkxzUe9d8P3cXSmRg4eY4": [(196494976, 0), (196520454, 1), (197246719, 2), (197494521, 3)],
        "5Lt5xXZG7bteZferQk9bsiiAS75JqGVPYcTbB8J6vvJK": [],
    },
    "mainnet": {
        "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA": [],
    }
}


class ClusterNotSupported(Exception):
    pass


class IdlPool:
    idls_per_program: Dict[str, Tuple[List[Tuple[int, Tuple[int, str]]], str, int]]

    def __init__(self, cluster):
        idl_dir = os.path.join(os.path.dirname(os.path.realpath(__file__)), f"idls/{cluster}")
        try:
            boundaries_per_program = IDL_VERSIONS[cluster]
        except KeyError:
            raise ClusterNotSupported(f"Cluster {cluster} is not supported")

        self.idls_per_program = {}

        for program_id in boundaries_per_program:
            # Find latest IDL
            sorted_idls = [int(os.path.basename(path).removesuffix(".json").removeprefix("marginfi-v")) for path in
                           glob.glob(f"{idl_dir}/{program_id}/marginfi-v*.json")]
            sorted_idls.sort()
            latest_idl_version = sorted_idls[-1]

            path = Path(f"{idl_dir}/{program_id}/marginfi-v{latest_idl_version}.json")
            latest_idl_raw = path.read_text()

            self.idls_per_program[program_id] = ([], latest_idl_raw, latest_idl_version)

            # Load all IDLs
            boundaries = boundaries_per_program[program_id]
            for boundary in boundaries:
                version_end_slot = boundary[0]
                idl_version = boundary[1]
                path = Path(f"{idl_dir}/{program_id}/marginfi-v{idl_version}.json")
                idl_raw = path.read_text()
                self.idls_per_program[program_id][0].append((version_end_slot, (idl_version, idl_raw)))

    def get_idl_for_slot(self, program_id: str, slot: int) -> Tuple[str, int]:
        idl_boundaries, latest_idl, latest_idl_version = self.idls_per_program[program_id]

        idl = None
        idl_version = None
        for version_end_slot, (current_idl_version, current_idl) in idl_boundaries:
            # todo: returns latest for upgrade slot, can throw if tx executed in same slot, before upgrade
            if version_end_slot > slot:
                idl = current_idl
                idl_version = current_idl_version
                break

        if idl is None:
            idl = latest_idl
            idl_version = latest_idl_version

        return idl, idl_version
