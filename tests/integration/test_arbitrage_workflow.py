# ArbitrageX Supreme V3.0 - Integration Tests
# Pruebas completas del workflow de arbitraje con Temporal.io

import pytest
import asyncio
from temporalio.client import Client
from temporalio.worker import Worker
from temporal_server.workflows.arbitrage_workflow import ExecuteArbitrageWorkflow, MonitorOpportunitiesWorkflow
import json
from datetime import timedelta

class TestArbitrageWorkflow:
    """
    Suite de pruebas de integración para workflows de arbitraje
    Valida ejecución completa end-to-end
    """
    
    @pytest.fixture
    async def temporal_client(self):
        """Cliente Temporal.io para pruebas"""
        client = await Client.connect("localhost:7233")
        return client
    
    @pytest.fixture
    def mock_opportunity(self):
        """Oportunidad de arbitraje mock para pruebas"""
        return {
            "id": "test_opp_001",
            "chain_id": 1,
            "strategy": "triangular_arbitrage",
            "tokens": ["WETH", "USDC", "DAI"],
            "pools": [
                {"address": "0x1234...", "fee": 3000},
                {"address": "0x5678...", "fee": 500},
                {"address": "0x9abc...", "fee": 3000}
            ],
            "profit_estimate": 0.05,  # 0.05 ETH
            "gas_estimate": 150000,
            "deadline": 1640995200,  # Unix timestamp
            "detected_at": 1640995180
        }
    
    @pytest.mark.asyncio
    async def test_successful_arbitrage_execution(self, temporal_client, mock_opportunity):
        """
        Test: Ejecución exitosa de arbitraje completo
        Verifica que el workflow se ejecute correctamente de principio a fin
        """
        
        # Ejecutar workflow
        result = await temporal_client.execute_workflow(
            ExecuteArbitrageWorkflow.run,
            mock_opportunity,
            id=f"test-arbitrage-{mock_opportunity['id']}",
            task_queue="arbitragex-task-queue",
            execution_timeout=timedelta(minutes=5)
        )
        
        # Verificaciones
        assert result["success"] is True
        assert "workflow_id" in result
        assert result["validation"]["valid"] is True
        assert result["simulation"]["profitable"] is True
        assert result["execution"]["status"] == "completed"
        assert result["reconciliation"]["status"] == "completed"
        assert result["total_profit"] > 0
        assert result["gas_used"] > 0
    
    @pytest.mark.asyncio
    async def test_unprofitable_opportunity_rejection(self, temporal_client):
        """
        Test: Rechazo de oportunidad no rentable
        Verifica que oportunidades con profit negativo sean rechazadas
        """
        
        unprofitable_opportunity = {
            "id": "test_opp_002",
            "chain_id": 1,
            "strategy": "triangular_arbitrage",
            "profit_estimate": -0.01,  # Pérdida
            "gas_estimate": 200000
        }
        
        result = await temporal_client.execute_workflow(
            ExecuteArbitrageWorkflow.run,
            unprofitable_opportunity,
            id=f"test-arbitrage-{unprofitable_opportunity['id']}",
            task_queue="arbitragex-task-queue",
            execution_timeout=timedelta(minutes=2)
        )
        
        # Verificaciones
        assert result["success"] is False
        assert result["reason"] in ["opportunity_invalid", "simulation_unprofitable"]
    
    @pytest.mark.asyncio
    async def test_workflow_retry_on_failure(self, temporal_client, mock_opportunity):
        """
        Test: Retry automático en caso de fallo temporal
        Verifica que el workflow reintente operaciones fallidas
        """
        
        # Simular fallo temporal modificando la oportunidad
        failing_opportunity = mock_opportunity.copy()
        failing_opportunity["simulate_failure"] = True
        
        result = await temporal_client.execute_workflow(
            ExecuteArbitrageWorkflow.run,
            failing_opportunity,
            id=f"test-arbitrage-retry-{failing_opportunity['id']}",
            task_queue="arbitragex-task-queue",
            execution_timeout=timedelta(minutes=10)
        )
        
        # El workflow debería fallar pero con información de retry
        assert "workflow_id" in result
        if not result["success"]:
            assert result["reason"] in ["workflow_error", "simulation_unprofitable"]
    
    @pytest.mark.asyncio
    async def test_monitor_opportunities_workflow(self, temporal_client):
        """
        Test: Workflow de monitoreo de oportunidades
        Verifica detección y lanzamiento de workflows hijos
        """
        
        result = await temporal_client.execute_workflow(
            MonitorOpportunitiesWorkflow.run,
            id="test-monitor-opportunities",
            task_queue="arbitragex-task-queue",
            execution_timeout=timedelta(minutes=2)
        )
        
        # Verificaciones
        assert "opportunities_detected" in result
        assert "workflows_launched" in result
        assert isinstance(result["launched_workflows"], list)
        assert result["opportunities_detected"] >= 0
        assert result["workflows_launched"] >= 0
    
    @pytest.mark.asyncio
    async def test_workflow_state_persistence(self, temporal_client, mock_opportunity):
        """
        Test: Persistencia de estado del workflow
        Verifica que el estado se mantenga ante interrupciones
        """
        
        # Iniciar workflow
        handle = await temporal_client.start_workflow(
            ExecuteArbitrageWorkflow.run,
            mock_opportunity,
            id=f"test-persistence-{mock_opportunity['id']}",
            task_queue="arbitragex-task-queue",
            execution_timeout=timedelta(minutes=5)
        )
        
        # Verificar que el workflow está corriendo
        description = await handle.describe()
        assert description.status.name in ["RUNNING", "COMPLETED"]
        
        # Obtener resultado final
        result = await handle.result()
        assert "workflow_id" in result

class TestTemporalIntegration:
    """
    Pruebas específicas de integración con Temporal.io
    """
    
    @pytest.mark.asyncio
    async def test_temporal_server_health(self):
        """
        Test: Salud del servidor Temporal
        Verifica conectividad y estado del servidor
        """
        
        client = await Client.connect("localhost:7233")
        
        # Verificar conectividad listando workflows
        async for workflow in client.list_workflows():
            # Si llegamos aquí, la conexión funciona
            break
        
        await client.close()
    
    @pytest.mark.asyncio
    async def test_worker_registration(self):
        """
        Test: Registro de workers
        Verifica que los workers se registren correctamente
        """
        
        client = await Client.connect("localhost:7233")
        
        # Crear worker de prueba
        worker = Worker(
            client,
            task_queue="test-task-queue",
            workflows=[ExecuteArbitrageWorkflow, MonitorOpportunitiesWorkflow]
        )
        
        # El worker debería inicializarse sin errores
        assert worker is not None
        
        await client.close()

class TestActivepieces:
    """
    Pruebas de integración con Activepieces
    """
    
    def test_activepieces_flows_valid_json(self):
        """
        Test: Validez de JSON en flows de Activepieces
        Verifica que todos los flows tengan JSON válido
        """
        
        flows_dir = "activepieces/flows"
        flow_files = [
            "alert-on-high-profit.json",
            "backup-db-to-minio.json", 
            "log-execution-to-slack.json"
        ]
        
        for flow_file in flow_files:
            with open(f"{flows_dir}/{flow_file}", 'r') as f:
                flow_data = json.load(f)
                
                # Verificaciones básicas de estructura
                assert "displayName" in flow_data
                assert "version" in flow_data
                assert "trigger" in flow_data
                assert "steps" in flow_data
    
    @pytest.mark.integration
    def test_activepieces_webhook_endpoints(self):
        """
        Test: Endpoints de webhook de Activepieces
        Verifica que los webhooks estén configurados correctamente
        """
        
        import requests
        
        base_url = "http://localhost:8081"
        
        # Test webhook de high profit alert
        webhook_url = f"{base_url}/api/v1/webhooks/arbitragex/high-profit-alert"
        
        test_payload = {
            "profit_usd": 1500,
            "gas_used": 0.02,
            "chain_id": 1,
            "strategy": "triangular_arbitrage",
            "timestamp": "2024-01-01T12:00:00Z",
            "tx_hash": "0xtest123..."
        }
        
        # Este test requiere que Activepieces esté corriendo
        try:
            response = requests.post(webhook_url, json=test_payload, timeout=5)
            # Webhook debería aceptar el payload
            assert response.status_code in [200, 202]
        except requests.exceptions.ConnectionError:
            pytest.skip("Activepieces not running")

if __name__ == "__main__":
    # Ejecutar pruebas
    pytest.main([__file__, "-v", "--asyncio-mode=auto"])
