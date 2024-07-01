import os
import random
import json

def generate_binary_corpus(num_files=100, max_size=4096):
    corpus_dir = os.path.join(os.path.dirname(__file__), "corpus/lend")
    os.makedirs(corpus_dir, exist_ok=True)
    
    for i in range(num_files):
        with open(f"{corpus_dir}/input_{i}.bin", "wb") as f:
            f.write(os.urandom(random.randint(1, max_size)))

generate_binary_corpus()
