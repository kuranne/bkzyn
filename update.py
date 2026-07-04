from pathlib import Path
import os
import toml
import subprocess as sp

def main():
    config_path = os.environ.get('XDG_CONFIG_HOME')
    if not config_path:
        config_path = f"{os.environ.get('HOME')}/.config"

    if not Path(config_path).exists:
        return 1

    toml_path = Path('./update.toml')
    toml_parser = toml.load(toml_path)
    configs = toml_parser.get('folders')
    bakup_dir = toml_parser.get('bakup_dir')
    
    for conf in configs:
        bak_list = []
        
        for item in Path(f"{bakup_dir}/{conf}").iterdir():
            bak_list.append(item.name)

        dir_path = Path(f"{config_path}/{conf}")
        if not dir_path.exists:
            print(f"Not found: {conf}, skipped")

        for item in dir_path.iterdir():
            if item.name in bak_list:
                cmd = ["cp", "-R", item, f"{bakup_dir}/{conf}/{item.name}"]
                sp.run(cmd)

    return 0

if __name__=="__main__":
    exit(main())
