<template>
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <h1 class="text-3xl font-bold mb-6">Compete</h1>
        <p class="mb-4 text-gray-700">Click on a service or flag in the treemap below to view details or submit a flag.</p>
        <div v-if="loading" class="flex justify-center items-center min-h-96">
            <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-gray-300"></div>
        </div>
        <div v-else-if="error" class="text-red-600 text-center">{{ error }}</div>
        <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-8">
            <div>
                <canvas ref="checkTreemap" height="400"></canvas>
            </div>
            <div>
                <canvas ref="flagTreemap" height="400"></canvas>
            </div>
        </div>
        <div v-if="selectedItem" class="fixed inset-0 bg-black/40 bg-blend-overlay flex items-center justify-center z-50" @click.self="selectedItem = null">
            <div class="bg-white rounded-lg shadow-lg p-8 max-w-md w-full relative">
                <button class="absolute top-2 right-2 text-gray-400 hover:text-gray-600"
                    @click="selectedItem = null">&times;</button>
                <h2 class="text-xl font-bold mb-2">{{ selectedItem.name }}</h2>
                <p class="mb-2">{{ selectedItem.description }}</p>
                <p class="mb-2 font-semibold">Points: {{ selectedItem.points }}</p>
                <p class="mb-4 font-semibold" v-if="selectedItem.interval">Interval: {{ selectedItem.interval }} seconds</p>
                <p class="mb-4 font-semibold" v-if="selectedItem.message">Status: {{ selectedItem.message }}</p>
                <div v-if="selectedItem.isFlag">
                    <div v-if="getFlagSolved(selectedItem.name)">
                        <span class="text-green-600 font-bold">Flag already solved!</span>
                    </div>
                    <div v-else>
                        <input v-model="flagInput" class="input-field w-full mb-2" placeholder="Enter flag..." :disabled="!competitionRunning" />
                        <button @click="redeemFlag" class="btn-primary w-full mb-2" :disabled="!competitionRunning" :class="{'opacity-50 cursor-not-allowed': !competitionRunning}">Submit Flag</button>
                        <div v-if="!competitionRunning" class="text-gray-500 text-center mb-2">Competition not running yet 😢</div>
                        <div v-if="redeemMessage" :class="redeemSuccess ? 'text-green-600' : 'text-red-600'">{{
                            redeemMessage }}</div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts">
import { defineComponent, ref, onMounted, computed } from 'vue';
import { Chart, Tooltip, Legend, Title, LinearScale } from 'chart.js';
import { TreemapController, TreemapElement, type TreemapScriptableContext } from 'chartjs-chart-treemap';
import { CompetitionStatus, type Check, type FlagCheck, type TeamCheckStatusResponse } from '@/types';
import apiService from '@/services/api';

// Register required Chart.js plugins and scales before treemap
Chart.register(Tooltip, Legend, Title, LinearScale, TreemapController, TreemapElement);

export default defineComponent({
    name: 'Compete',
    setup() {
        const checks = ref<Check[]>([]);
        const flagChecks = ref<FlagCheck[]>([]);
        const checkStatus = ref<TeamCheckStatusResponse>({ checks: [], flag_checks: [] });
        const teamId = ref<number | null>(null);
        const loading = ref(true);
        const error = ref('');
        const selectedItem = ref<any>(null);
        const flagInput = ref('');
        const redeemMessage = ref('');
        const redeemSuccess = ref<boolean | null>(null);
        const competitionStatus = ref<CompetitionStatus | null>(null);

        const competitionRunning = computed(() => 
        {
            return competitionStatus.value && competitionStatus.value.toString() === "Active";
        });

        const getCurrentTeamId = async () => {
            const user = await apiService.getCurrentUser();
            return user.teamId || null;
        };

        const fetchData = async () => {
            loading.value = true;
            error.value = '';
            try {
                const [checkResp, teamIdVal, comp] = await Promise.all([
                    apiService.getChecks(),
                    getCurrentTeamId(),
                    apiService.getCompetition()
                ]);
                checks.value = checkResp.checks;
                flagChecks.value = checkResp.flag_checks;
                teamId.value = teamIdVal;
                console.log('comp.status:', comp.status);
                competitionStatus.value = comp.status;
                if (teamId.value) {
                    checkStatus.value = await apiService.getCheckStatus(teamId.value);
                }
            } catch (e: any) {
                error.value = 'Failed to load checks.';
            } finally {
                loading.value = false;
            }
        };

        onMounted(fetchData);

        function getCheckPassing(name: string, isFlag = false) {
            if (!checkStatus.value) return false;
            if (isFlag) {
                return checkStatus.value.flag_checks.find(f => f.name === name)?.passing;
            } else {
                return checkStatus.value.checks.find(c => c.name === name)?.passing;
            }
        }

        function getFlagSolved(name: string) {
            return checkStatus.value.flag_checks.find(f => f.name === name)?.passing === true;
        }

        function colorFromRaw(ctx: TreemapScriptableContext, isFlag = false) {
            if (ctx.raw && ctx.raw.g) {
                const name = ctx.raw.g;
                const passing = getCheckPassing(name, isFlag);
                return passing ? 'green' : 'red';
            }
            return 'gray';
        }

        function handleTreemapClick(evt: any, chart: any, items: any[], isFlag: boolean) {
            if (!items.length) return;
            const elem = items[0].element.$context;
            let itemData;
            let itemDataStatus;
            if (isFlag) {
                // Find the flag check by name
                const flagName = elem.raw && elem.raw.g ? elem.raw.g : null;
                itemData = flagChecks.value.find(f => f.name === flagName);
                selectedItem.value = { ...itemData, isFlag };

            } else {
                // Find the check by name
                const checkName = elem.raw && elem.raw.g ? elem.raw.g : null;
                itemData = checks.value.find(c => c.name === checkName);
                itemDataStatus = checkStatus.value.checks.find(c => c.name === checkName);
                selectedItem.value = { ...itemData, ...itemDataStatus, isFlag };

            }
            if (!itemData) return;
            flagInput.value = '';
            redeemMessage.value = '';
            redeemSuccess.value = null;
        }

        async function redeemFlag() {
            if (!selectedItem.value || !competitionRunning.value) return;
            try {
                const resp = await apiService.redeemFlag({
                    flag: flagInput.value,
                    flagCheckName: selectedItem.value.name
                });
                redeemMessage.value = resp.message;
                redeemSuccess.value = resp.success;
                if (resp.success) {
                    await fetchData();
                }
            } catch (e: any) {
                redeemMessage.value = e?.response?.data?.message || 'Error submitting flag.';
                redeemSuccess.value = false;
            }
        }

        let checkChart: Chart | null = null;
        let flagChart: Chart | null = null;
        const checkTreemap = ref<HTMLCanvasElement | null>(null);
        const flagTreemap = ref<HTMLCanvasElement | null>(null);

        onMounted(() => {
            fetchData().then(() => {
                if (checkTreemap.value) {
                    checkChart = new Chart(checkTreemap.value, {
                        type: 'treemap',
                        data: {
                            datasets: [
                                {
                                    tree: checks.value.map(c => ({
                                        _check: c.name,
                                        key: c.points / c.interval,
                                        points: c.points,
                                        interval : c.interval,
                                    })),
                                    data: [], //fix for typescript
                                    label : "Importance",
                                    labels : {
                                        display: true,
                                        formatter: (ctx) => {
                                            let name = ctx.raw && ctx.raw.g ? ctx.raw.g : '';
                                            return name
                                        }
                                    },
                                    key : "key",
                                    groups : ["_check"],
                                    backgroundColor: (ctx) => colorFromRaw(ctx, false)
                                }
                            ]
                        },
                        options: {
                            plugins: {
                                title: { display: true, text: 'Service Checks' },
                                legend: { display: false },
                                tooltip: {
                                    enabled: false
                                }
                            },
                            onClick: (evt: any, items: any, chart: any) => handleTreemapClick(evt, chart, items, false),
                            parsing: false
                        }
                    });
                }
                if (flagTreemap.value) {
                    flagChart = new Chart(flagTreemap.value, {
                        type: 'treemap',
                        data: {
                            datasets: [
                                {
                                    label: 'Flag Checks',
                                    labels: {
                                        display: true,
                                        formatter: (ctx) => {
                                            let name = ctx.raw && ctx.raw.g ? ctx.raw.g : '';
                                            return name
                                        }
                                    },
                                    tree: flagChecks.value.map(f => ({
                                        _check: f.name,
                                        key: f.points
                                    })),
                                    data: [], //fix for typescript
                                    key : "key",
                                    groups : ['_check'],
                                    backgroundColor: (ctx) => colorFromRaw(ctx, true)
                                }
                            ]
                        },
                        options: {
                            plugins: {
                                title: { display: true, text: 'Flag Checks' },
                                legend: { display: false }
                            },
                            onClick: (evt: any, items: any, chart: any) => handleTreemapClick(evt, chart, items, true),
                            parsing: false
                        }
                    });
                }
            });
        });

        return {
            checks,
            flagChecks,
            checkStatus,
            teamId,
            loading,
            error,
            selectedItem,
            flagInput,
            redeemMessage,
            redeemSuccess,
            fetchData,
            handleTreemapClick,
            redeemFlag,
            checkTreemap,
            flagTreemap,
            getFlagSolved,
            competitionRunning
        };
    }
});
</script>

<style scoped>
.input-field {
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    padding: 0.5rem;
    font-size: 1rem;
}

.btn-primary {
    background: #2563eb;
    color: white;
    border-radius: 0.375rem;
    padding: 0.5rem 1rem;
    font-weight: 600;
    transition: background 0.2s;
}

.btn-primary:hover {
    background: #1d4ed8;
}
</style>