FROM gcr.io/dataflow-templates-base/python39-template-launcher-base

ARG JOB_DIRECTORY
ARG WORKDIR=/dataflow/template
RUN mkdir -p ${WORKDIR}
WORKDIR ${WORKDIR}

COPY ${JOB_DIRECTORY}/requirements.txt .
COPY ${JOB_DIRECTORY}/job.py .
COPY idls/ idls/
COPY dataflow_etls/ dataflow_etls/

ENV FLEX_TEMPLATE_PYTHON_REQUIREMENTS_FILE="/${WORKDIR}/requirements.txt"
ENV FLEX_TEMPLATE_PYTHON_PY_FILE="/${WORKDIR}/job.py"

# We could get rid of installing libffi-dev and git, or we could leave them.
RUN apt-get update \
    && apt-get install -y libffi-dev git \
    && rm -rf /var/lib/apt/lists/* \
    # Upgrade pip and install the requirements.
    && pip install --no-cache-dir --upgrade pip \
    && pip install -U apache-beam==2.44.0 \
    && pip install --no-cache-dir -r $FLEX_TEMPLATE_PYTHON_REQUIREMENTS_FILE \
    # Download the requirements to speed up launching the Dataflow job.
    && pip download --no-cache-dir --dest /tmp/dataflow-etls-requirements-cache -r $FLEX_TEMPLATE_PYTHON_REQUIREMENTS_FILE

# Since we already downloaded all the dependencies, there's no need to rebuild everything.
ENV PIP_NO_DEPS=True
