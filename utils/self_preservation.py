import os
import hashlib
import shutil
from typing import Dict, Any, List
from loguru import logger
from .base_agent import BaseAgent

class SelfPreservation:
    def __init__(self, quarantine_dir: str = "quarantine", backup_dir: str = "backups"):
        self.quarantine_dir = quarantine_dir
        self.backup_dir = backup_dir
        os.makedirs(quarantine_dir, exist_ok=True)
        os.makedirs(backup_dir, exist_ok=True)
        self.threat_signatures = self._load_threat_signatures()

    def _load_threat_signatures(self) -> List[str]:
        # Placeholder for threat signatures (hashes, patterns)
        return [
            "malicious_pattern_1",
            "injection_attack_hash",
            # Add more
        ]

    def detect_threat(self, data: Any) -> bool:
        # Simple threat detection
        if isinstance(data, str):
            for sig in self.threat_signatures:
                if sig in data:
                    return True
        elif isinstance(data, dict):
            for value in data.values():
                if self.detect_threat(value):
                    return True
        return False

    def quarantine(self, file_path: str):
        if os.path.exists(file_path):
            dest = os.path.join(self.quarantine_dir, os.path.basename(file_path))
            shutil.move(file_path, dest)
            logger.warning(f"Quarantined {file_path} to {dest}")

    def mitigate(self, threat_data: Any) -> Any:
        # Sanitize or neutralize threat
        if isinstance(threat_data, str):
            # Remove suspicious patterns
            sanitized = threat_data.replace("malicious", "[REMOVED]").replace("injection", "[REMOVED]")
            return sanitized
        return threat_data

    def patch_and_heal(self, system_component: str, issue: str, llm_clients: Dict[str, Any]):
        # Use LLMs to generate patches
        prompt = f"Generate a patch for {system_component} issue: {issue}"
        patches = []
        for name, client in llm_clients.items():
            try:
                response = client.query(prompt)
                patches.append(response)
            except:
                pass
        # Apply the best patch (placeholder)
        logger.info(f"Generated patches: {patches}")
        # In practice, apply the patch

    def backup_system(self):
        # Backup critical files
        critical_files = ["main.py", "requirements.txt"]  # Add more
        for file in critical_files:
            if os.path.exists(file):
                shutil.copy(file, os.path.join(self.backup_dir, file))
        logger.info("System backup completed")

    def handle_intrusion(self, intrusion_data: Dict[str, Any], llm_clients: Dict[str, Any]):
        logger.error(f"Intrusion detected: {intrusion_data}")
        # Quarantine affected files
        affected_files = intrusion_data.get('affected_files', [])
        for file in affected_files:
            self.quarantine(file)
        # Mitigate
        mitigated = self.mitigate(intrusion_data)
        # Patch
        self.patch_and_heal("system", str(intrusion_data), llm_clients)
        # Backup
        self.backup_system()