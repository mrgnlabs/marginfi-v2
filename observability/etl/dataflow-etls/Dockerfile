FROM gcr.io/dataflow-templates-base/python39-template-launcher-base

ARG JOB_DIRECTORY
ARG WORKDIR=/dataflow/template
RUN mkdir -p ${WORKDIR}
WORKDIR ${WORKDIR}

COPY setup.py .
COPY MANIFEST.in .

ENV FLEX_TEMPLATE_PYTHON_PY_FILE="/${WORKDIR}/job.py"
ENV FLEX_TEMPLATE_PYTHON_SETUP_FILE="/${WORKDIR}/setup.py"

# Upgrade pip and install the requirements.
RUN pip install --no-cache-dir --upgrade pip \
    && pip install --no-cache-dir . \
    # Download the requirements to speed up launching the Dataflow job.
    && pip download --no-cache-dir --dest /tmp/dataflow-etls-requirements-cache .

COPY dataflow_etls/ dataflow_etls/
COPY ${JOB_DIRECTORY}/job.py .

# Since we already downloaded all the dependencies, there's no need to rebuild everything.
ENV PIP_NO_DEPS=True
