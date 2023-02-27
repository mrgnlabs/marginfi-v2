import setuptools

setuptools.setup(
    name='dataflow_etls',
    version='0.1.0',
    install_requires=[
        'anchorpy==0.16.0',
        'apache-beam[gcp]==2.45.0',
        "based58==0.1.1",
        "solders==0.14.4 ",
    ],
    packages=setuptools.find_packages(),
    include_package_data=True
)
